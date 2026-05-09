// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Description
//!
//! This module implements lazy identity mapping for physical memory accesses.
//!
//! Only the kernel process has physical memory identity-mapped into its virtual address space.
//! At boot, page tables are allocated from BSS for kernel code/data/bss/kpool/modules. When
//! the kernel later needs to access a physical frame not yet mapped (for example, during IPC
//! copies), this module lazily allocates a BSS page table and fills the PTE in the kernel PD.
//!
//! User processes do not get identity-mapping updates. When kernel code running in a user
//! address space needs to perform a physical-memory operation, it must temporarily switch CR3
//! to the kernel address space through the local CR3-switch helper.

//==================================================================================================
// Imports
//==================================================================================================

use super::page_table_allocator::PAGE_TABLE_ALLOCATOR;
use crate::hal::{
    arch::x86::{
        fast_memcpy,
        fast_memset,
    },
    mem::{
        Address,
        PageAligned,
        PageDirectoryAddress,
        PhysicalAddress,
    },
};
use ::arch::{
    cpu::cr3::Cr3Register,
    mem::{
        self,
        paging::{
            self,
            AccessedFlag,
            DirtyFlag,
            FrameNumber,
            PageCacheDisableFlag,
            PageDirectoryEntry,
            PageDirectoryEntryFlags,
            PageSizeFlag,
            PageTableEntry,
            PageTableEntryFlags,
            PageWriteThroughFlag,
            PresentFlag,
            PteWord,
            ReadWriteFlag,
            Table,
            TableIndex,
            UserSupervisorFlag,
        },
        PAGE_TABLE_LENGTH,
    },
};
use ::core::sync::atomic::{
    AtomicU32,
    AtomicUsize,
    Ordering,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use arch::mem::PAGE_ALIGNMENT;
//==================================================================================================
// Global State
//==================================================================================================

/// Physical address of the kernel page directory (set once during boot by [`init`]).
static KERNEL_PD_PADDR: AtomicUsize = AtomicUsize::new(0);

/// Raw value of the kernel CR3 register for address-space switching.
static KERNEL_CR3: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Public API
//==================================================================================================

///
/// # Description
///
/// Records the kernel page-directory and root paging-structure physical addresses used by the
/// lazy identity mapper.
///
/// # Parameters
///
/// - `kernel_pd_paddr`: Physical address of the kernel page directory.
/// - `kernel_cr3`: CR3 register value for the kernel root paging structure used for CR3 switching.
///
/// # Returns
///
/// This function returns no value.
///
/// # Notes
///
/// On x86, `kernel_cr3` equals `kernel_pd_paddr` (the page directory is the CR3 root).
pub(crate) fn init(kernel_pd_paddr: PageDirectoryAddress, kernel_cr3: Cr3Register) {
    KERNEL_PD_PADDR.store(kernel_pd_paddr.into_raw_value(), Ordering::Release);
    KERNEL_CR3.store(kernel_cr3.into_u32(), Ordering::Release);
}

///
/// # Description
///
/// Copies bytes between two memory regions, after ensuring that both ranges are identity-mapped in
/// the kernel address space.
///
/// # Parameters
///
/// - `dst`: Destination physical address.
/// - `src`: Source physical address.
/// - `size`: Number of bytes to copy.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::BadAddress`]: One of the physical ranges is invalid or overflows.
/// - Any error propagated by the lazy identity mapper while preparing the ranges.
///
/// # Notes
///
/// If `size == 0`, this function is a no-op and returns success.
///
pub(crate) fn memcpy(dst: *mut u8, src: *const u8, size: usize) -> Result<(), Error> {
    // Check if copy size is zero.
    if size == 0 {
        return Ok(());
    }

    let dst_start: usize = dst as usize;
    let src_start: usize = src as usize;
    let dst_end: usize = dst_start.checked_add(size).ok_or_else(|| {
        error!("memcpy(): destination range overflows (dst={dst_start:#x}, size={size:#x})");
        Error::new(ErrorCode::BadAddress, "memcpy(): destination range overflows")
    })?;
    let src_end: usize = src_start.checked_add(size).ok_or_else(|| {
        error!("memcpy(): source range overflows (src={src_start:#x}, size={size:#x})");
        Error::new(ErrorCode::BadAddress, "memcpy(): source range overflows")
    })?;

    // Check if copy ranges overlap.
    if (dst_start..dst_end).contains(&src_start) || (src_start..src_end).contains(&dst_start) {
        error!(
            "memcpy(): source and destination ranges overlap (dst={dst_start:#x}, \
             src={src_start:#x}, size={size:#x})"
        );
        return Err(Error::new(
            ErrorCode::BadAddress,
            "memcpy(): source and destination ranges overlap",
        ));
    }

    with_kernel_address_space(|| {
        let src_addr: PhysicalAddress = PhysicalAddress::from_raw_value(src as usize)?;
        let (src_start, src_size) = page_aligned_cover(src_addr, size)?;
        ensure_identity_mapped_range(src_start, src_size)?;
        let dst_addr: PhysicalAddress = PhysicalAddress::from_raw_value(dst as usize)?;
        let (dst_start, dst_size) = page_aligned_cover(dst_addr, size)?;
        ensure_identity_mapped_range(dst_start, dst_size)?;

        // SAFETY: both `src` and `dst` are identity-mapped (virtual == physical) and
        // valid for `size` bytes. The overlap check above guarantees non-overlapping ranges.
        unsafe {
            fast_memcpy(dst, src, size);
        }
        Ok(())
    })
}

///
/// # Description
///
/// Fills bytes in a memory range with a byte value, after ensuring that the full target range is
/// identity-mapped in the kernel address space.
///
/// # Parameters
///
/// - `base`: Starting physical address of the target range.
/// - `value`: Byte value to fill.
/// - `size`: Number of bytes to fill.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::BadAddress`]: The target range is invalid or overflows.
/// - Any error propagated by the lazy identity mapper while preparing the range.
///
/// # Notes
///
/// If `size == 0`, this function is a no-op and returns success.
///
pub(crate) fn memset(base: *mut u8, value: u8, size: usize) -> Result<(), Error> {
    // Check if fill size is zero.
    if size == 0 {
        return Ok(());
    }

    let base_start: usize = base as usize;
    // Check if fill range overflows.
    base_start.checked_add(size).ok_or_else(|| {
        error!("memset(): target range overflows (base={base_start:#x}, size={size:#x})");
        Error::new(ErrorCode::BadAddress, "memset(): target range overflows")
    })?;

    with_kernel_address_space(|| {
        let base_addr: PhysicalAddress = PhysicalAddress::from_raw_value(base as usize)?;
        let (base_start, base_size) = page_aligned_cover(base_addr, size)?;
        ensure_identity_mapped_range(base_start, base_size)?;

        // SAFETY: `base` is identity-mapped (virtual == physical) and valid for
        // writes of `size` bytes.
        unsafe {
            fast_memset(base, value, size);
        }
        Ok(())
    })
}

/// RAII guard that restores the original CR3 value when dropped.
struct Cr3Guard(Cr3Register);

impl Drop for Cr3Guard {
    ///
    /// # Description
    ///
    /// Restores the previously active CR3 value saved in this guard.
    ///
    fn drop(&mut self) {
        // SAFETY: caller runs at privilege level 0; CR3 holds a valid PD address.
        unsafe {
            self.0.write();
        }
    }
}

///
/// # Description
///
/// Temporarily switches CR3 to the kernel address space, executes `f`, and restores the previous
/// CR3 value on return.
///
/// This gives `f` access to kernel identity mappings while preserving the caller's original
/// address-space context through RAII restoration.
///
/// # Parameters
///
/// - `f`: Closure to execute while CR3 points to the kernel address space.
///
/// # Returns
///
/// Returns the value produced by `f`.
///
/// # Interrupt Safety
///
/// The caller must ensure that interrupts are disabled or that interrupt handlers are
/// CR3-agnostic. If an interrupt fires while CR3 points to the kernel address space,
/// the handler will execute in the kernel address space rather than the original one.
///
pub(crate) fn with_kernel_address_space<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let kernel_cr3_raw: u32 = KERNEL_CR3.load(Ordering::Acquire);
    // Check if the kernel CR3 has been initialized.
    if kernel_cr3_raw == 0 {
        return f();
    }

    // SAFETY: caller runs at privilege level 0.
    let old_cr3: Cr3Register = unsafe { Cr3Register::read() };

    // If we are already in the kernel address space, no switch needed.
    // SAFETY: `kernel_cr3_raw` was produced by `Cr3Register::into_u32()` during `init()`,
    // so it is guaranteed to have no reserved bits set.
    let kernel_cr3: Cr3Register = unsafe { Cr3Register::from_u32_unchecked(kernel_cr3_raw) };
    if old_cr3 == kernel_cr3 {
        return f();
    }

    // Switch to kernel address space. The Cr3Guard restores `old_cr3` on drop.
    let _guard = Cr3Guard(old_cr3);
    // SAFETY: caller runs at privilege level 0; kernel_cr3 holds a valid PD address.
    unsafe {
        kernel_cr3.write();
    }

    f()
}

///
/// # Description
///
/// Computes the smallest page-aligned range that fully covers `[addr, addr + size)`.
///
/// # Parameters
///
/// - `addr`: Starting physical address of the target range.
/// - `size`: Number of bytes in the target range (must be > 0).
///
/// # Returns
///
/// Upon success, a tuple of the page-aligned start address and the page-aligned size is returned.
/// Upon failure, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::BadAddress`]: The range overflows or contains an invalid physical address.
///
fn page_aligned_cover(
    addr: PhysicalAddress,
    size: usize,
) -> Result<(PageAligned<PhysicalAddress>, usize), Error> {
    debug_assert!(size > 0);
    let start: PageAligned<PhysicalAddress> =
        PageAligned::from_address(addr.align_down(PAGE_ALIGNMENT)?)?;
    // Compute the exclusive end as a raw `usize` rather than a `PhysicalAddress` because the
    // exclusive end may equal `MEMORY_SIZE`, which is out of bounds for `PhysicalAddress`.
    let raw: usize = addr.into_raw_value();
    let exclusive_end: usize = raw.checked_add(size).ok_or_else(|| {
        error!("page_aligned_cover(): physical range overflow (addr={raw:#x}, size={size:#x})");
        Error::new(ErrorCode::BadAddress, "physical range overflow")
    })?;
    // Round exclusive end up to the next page boundary.
    let aligned_end: usize = exclusive_end
        .checked_next_multiple_of(mem::PAGE_SIZE)
        .ok_or_else(|| {
            error!("page_aligned_cover(): aligned end overflow (exclusive_end={exclusive_end:#x})");
            Error::new(ErrorCode::BadAddress, "aligned end overflow")
        })?;
    let aligned_size: usize = aligned_end - start.into_raw_value();
    Ok((start, aligned_size))
}

///
/// # Description
///
/// Ensures that every page in `[start, start + size)` is identity-mapped in the kernel
/// page directory.
///
/// # Parameters
///
/// - `start`: Page-aligned starting physical address of the target range.
/// - `size`: Number of bytes in the target range (must be a multiple of [`PAGE_SIZE`]).
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::BadAddress`]: The range contains an invalid physical address.
///
/// # Notes
///
/// If `size == 0`, this function is a no-op and returns success.
fn ensure_identity_mapped_range(
    start: PageAligned<PhysicalAddress>,
    size: usize,
) -> Result<(), Error> {
    debug_assert!(size.is_multiple_of(mem::PAGE_SIZE));

    if size == 0 {
        return Ok(());
    }

    let start_raw: usize = start.into_raw_value();
    let num_pages: usize = size / mem::PAGE_SIZE;
    for i in 0..num_pages {
        let page_addr: PageAligned<PhysicalAddress> =
            PageAligned::from_raw_value(start_raw + i * mem::PAGE_SIZE)?;
        identity_map_page(page_addr)?;
    }

    Ok(())
}

///
/// # Description
///
/// Ensures that a page table exists for the given PDE index in the kernel page directory. If the
/// PDE is already present, returns the physical address of the existing page table. Otherwise,
/// allocates a new page table from the BSS pool, installs the PDE, and returns the new page
/// table's physical address.
///
/// # Parameters
///
/// - `pd`: The kernel page directory table.
/// - `pde_idx`: The page directory index to check.
///
/// # Returns
///
/// Upon success, the physical address of the page table is returned. Upon failure, an error is
/// returned instead.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`]: Failed to read the PDE.
/// - [`ErrorCode::OutOfMemory`]: No BSS page table slots available.
/// - [`ErrorCode::BadAddress`]: The allocated page table frame number is out of range.
///
fn ensure_pt(pd: Table<PageDirectoryEntry>, pde_idx: TableIndex) -> Result<usize, Error> {
    let pde: PageDirectoryEntry = unsafe { pd.read(pde_idx) }.ok_or_else(|| {
        let reason: &str = "invalid PDE read from kernel PD";
        error!("ensure_pt(): {reason}");
        Error::new(ErrorCode::InvalidArgument, reason)
    })?;

    if pde.is_present() {
        return Ok(pde.frame_address());
    }

    // No PT exists — allocate one from the BSS pool (zeroed = all PTEs absent).
    // SAFETY: atomic bump allocator ensures exclusive access to the slot;
    // BSS is zero-initialized, so assume_init_mut() is sound for integer arrays.
    let slot: &'static mut [PteWord; PAGE_TABLE_LENGTH] = unsafe {
        PAGE_TABLE_ALLOCATOR
            .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
            .map_err(|e| {
                error!("ensure_pt(): page table allocation failed: {}", e);
                Error::new(ErrorCode::OutOfMemory, "BSS page table allocation failed")
            })?
            .assume_init_mut()
    };
    let pt_paddr: usize = slot.as_ptr() as usize;

    // Install the PDE with type-safe flag constructors.
    let pt_frame: FrameNumber =
        FrameNumber::from_raw_value(pt_paddr / mem::PAGE_SIZE).ok_or_else(|| {
            let reason: &str = "BSS page table frame number out of range";
            error!("ensure_pt(): {reason}");
            Error::new(ErrorCode::BadAddress, reason)
        })?;
    let new_pde: PageDirectoryEntry = PageDirectoryEntry::new(
        PageDirectoryEntryFlags::new(
            PresentFlag::Present,
            ReadWriteFlag::ReadWrite,
            UserSupervisorFlag::Supervisor,
            PageWriteThroughFlag::NotWriteThrough,
            PageCacheDisableFlag::CacheEnabled,
            AccessedFlag::NotAccessed,
            DirtyFlag::NotDirty,
            PageSizeFlag::Standard,
        ),
        pt_frame,
    );
    // SAFETY: the kernel PD is identity-mapped.
    unsafe { pd.write(pde_idx, new_pde) };

    Ok(pt_paddr)
}

///
/// # Description
///
/// Ensures that a page table entry for the given index is identity-mapped. If the PTE is already
/// present, this function is a no-op. Otherwise, it creates a new identity-mapped PTE and
/// invalidates the corresponding TLB entry.
///
/// # Parameters
///
/// - `pt`: The page table to write into.
/// - `pte_idx`: The page table index to check.
/// - `phys_addr`: The physical address to identity-map (used as both frame source and TLB target).
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`]: Failed to read the PTE.
/// - [`ErrorCode::BadAddress`]: The frame number is out of range.
///
fn ensure_pte(
    pt: Table<PageTableEntry>,
    pte_idx: TableIndex,
    phys_addr: usize,
) -> Result<(), Error> {
    let pte: PageTableEntry = unsafe { pt.read(pte_idx) }.ok_or_else(|| {
        let reason: &str = "invalid PTE read from page table";
        error!("ensure_pte(): {reason}");
        Error::new(ErrorCode::InvalidArgument, reason)
    })?;

    if pte.is_present() {
        return Ok(());
    }

    // Fill the PTE with an identity mapping.
    let frame: FrameNumber =
        FrameNumber::from_raw_value(phys_addr / mem::PAGE_SIZE).ok_or_else(|| {
            let reason: &str = "frame number out of range";
            error!("ensure_pte(): {reason}");
            Error::new(ErrorCode::BadAddress, reason)
        })?;
    let new_pte: PageTableEntry = PageTableEntry::new(
        PageTableEntryFlags::new(
            PresentFlag::Present,
            ReadWriteFlag::ReadWrite,
            UserSupervisorFlag::Supervisor,
            PageWriteThroughFlag::NotWriteThrough,
            PageCacheDisableFlag::CacheEnabled,
            AccessedFlag::NotAccessed,
            DirtyFlag::NotDirty,
        ),
        frame,
    );
    // SAFETY: the PT is identity-mapped.
    unsafe { pt.write(pte_idx, new_pte) };

    // Invalidate the TLB entry for this page.
    unsafe { paging::invlpg(phys_addr) };

    Ok(())
}

///
/// # Description
///
/// Propagates the kernel PDE at `pde_idx` to the currently-active page directory (if it differs
/// from the kernel PD and lacks that PDE). This enables kernel code running in a user address
/// space to access identity-mapped memory without faulting.
///
fn propagate_pde_to_current_pd(kernel_pd_paddr: usize, pde_idx: TableIndex) -> Result<(), Error> {
    // SAFETY: running in supervisor mode.
    let current_cr3: Cr3Register = unsafe { Cr3Register::read() };
    let current_pd_paddr: usize = current_cr3.paging_structure_base_address.address() as usize;

    // Nothing to do if we're already in the kernel address space.
    if current_pd_paddr == kernel_pd_paddr {
        return Ok(());
    }
    if current_pd_paddr == 0 {
        trace!("propagate_pde_to_current_pd(): CR3 is zero (early boot?), skipping");
        return Ok(());
    }

    // Read the kernel PDE.
    let kernel_pd: Table<PageDirectoryEntry> = unsafe { Table::from_address(kernel_pd_paddr) };
    let kernel_pde: PageDirectoryEntry = unsafe { kernel_pd.read(pde_idx) }
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "invalid PDE index"))?;

    if !kernel_pde.is_present() {
        return Ok(());
    }

    // Only propagate if the current PD doesn't already have a present PDE at this index.
    let current_pd: Table<PageDirectoryEntry> = unsafe { Table::from_address(current_pd_paddr) };
    let current_pde: PageDirectoryEntry = unsafe { current_pd.read(pde_idx) }
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "invalid PDE index"))?;

    if !current_pde.is_present() {
        unsafe { current_pd.write(pde_idx, kernel_pde) };
    }

    Ok(())
}

///
/// # Description
///
/// Identity-maps a single page in the kernel page directory.
///
/// If the target PTE is already present, this function is a no-op. If the PDE is absent, a new
/// page table is allocated from the BSS pool and installed before the identity-mapped PTE is
/// created.
///
/// # Parameters
///
/// - `phys_addr`: Page-aligned physical address to identity-map.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`]: Failed to read a valid paging entry.
///
/// # Notes
///
/// If the lazy mapper has not been initialized yet (boot page tables still active), this function
/// is a no-op and returns success.
pub(crate) fn identity_map_page(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
    let phys_addr: usize = phys_addr.into_raw_value();

    let pd_paddr: usize = KERNEL_PD_PADDR.load(Ordering::Acquire);
    // Check if the lazy mapper has been initialized.
    if pd_paddr == 0 {
        return Ok(());
    }

    let pde_idx: TableIndex = paging::pd_index(phys_addr);
    let pte_idx: TableIndex = paging::pt_index(phys_addr);

    // SAFETY: the PD is identity-mapped.
    let pd: Table<PageDirectoryEntry> = unsafe { Table::from_address(pd_paddr) };
    let pt_paddr: usize = ensure_pt(pd, pde_idx)?;

    // Eagerly propagate the PDE to the currently-active page directory (if different from the
    // kernel PD). This ensures that kernel code running in a user address space can immediately
    // access the identity-mapped page without faulting.
    propagate_pde_to_current_pd(pd_paddr, pde_idx)?;

    // SAFETY: the PT is identity-mapped (BSS-backed).
    let pt: Table<PageTableEntry> = unsafe { Table::from_address(pt_paddr) };
    ensure_pte(pt, pte_idx, phys_addr)
}

///
/// # Description
///
/// Propagates kernel identity-mapping PDEs for the given physical address range into a target page
/// directory. This ensures that the target PD (typically a user process PD) can access
/// identity-mapped kernel memory (e.g., kernel stacks) without faulting.
///
/// For each page in `[start, start + size)`, copies the corresponding PDE from the kernel PD into
/// the target PD if the kernel PD has a present PDE and the target PD does not.
///
/// # Parameters
///
/// - `target_pd_paddr`: Physical address of the target page directory.
/// - `start`: Page-aligned start of the physical address range.
/// - `size`: Size of the range in bytes (must be page-aligned and > 0).
///
/// # Returns
///
/// Upon success, `Ok(())`. Upon failure, an error is returned.
///
pub(crate) fn propagate_kernel_pdes(
    target_pd_paddr: usize,
    start: usize,
    size: usize,
) -> Result<(), Error> {
    let kernel_pd_paddr: usize = KERNEL_PD_PADDR.load(Ordering::Acquire);
    if kernel_pd_paddr == 0 {
        return Ok(());
    }

    // SAFETY: both PDs are identity-mapped (BSS-backed or in kernel memory).
    let kernel_pd: Table<PageDirectoryEntry> = unsafe { Table::from_address(kernel_pd_paddr) };
    let target_pd: Table<PageDirectoryEntry> = unsafe { Table::from_address(target_pd_paddr) };

    // Iterate over all PDE indices covered by [start, start + size).
    let end: usize = start + size;
    let mut addr: usize = start;
    let mut last_pde_idx: Option<TableIndex> = None;

    while addr < end {
        let pde_idx: TableIndex = paging::pd_index(addr);

        // Skip if we already processed this PDE index.
        if last_pde_idx == Some(pde_idx) {
            addr += mem::PAGE_SIZE;
            continue;
        }
        last_pde_idx = Some(pde_idx);

        // Read kernel PDE.
        let kernel_pde: PageDirectoryEntry = unsafe { kernel_pd.read(pde_idx) }
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "invalid PDE index"))?;

        if !kernel_pde.is_present() {
            addr += mem::PAGE_SIZE;
            continue;
        }

        // Read target PDE — only propagate if not already present.
        let target_pde: PageDirectoryEntry = unsafe { target_pd.read(pde_idx) }
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "invalid PDE index"))?;

        if !target_pde.is_present() {
            unsafe { target_pd.write(pde_idx, kernel_pde) };
        }

        addr += mem::PAGE_SIZE;
    }

    Ok(())
}
