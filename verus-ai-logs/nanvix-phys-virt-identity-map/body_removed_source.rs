// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Description
//!
//! This module implements identity mapping for physical memory accesses.
//!
//! Only the kernel process has physical memory identity-mapped into its virtual address space.
//! At boot, page tables are allocated from BSS for kernel code/data/bss/modules. During
//! [`init`], page tables for every PDE index in `[0, MEMORY_SIZE)` are pre-allocated from the
//! BSS pool, covering all physical memory. If a frame is later allocated outside these
//! pre-covered ranges, [`identity_map_page`] lazily installs a page table entry (and, when
//! necessary, a new page table from the BSS pool) for the corresponding PDE.
//!
//! When a new user address space is created, [`sync_kernel_pdes`] copies all present kernel
//! identity-mapping PDEs into the new page directory in a single pass.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("identity_map.spec.rs");
#[cfg(verus_keep_ghost)]
include!("identity_map.proof.rs");

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
        PAGE_ALIGNMENT,
        PAGE_TABLE_LENGTH,
    },
};
use ::config::kernel::MEMORY_SIZE;
use ::core::sync::atomic::{
    AtomicU32,
    AtomicUsize,
    Ordering,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

// Compile-time check: the number of identity-mapping PDEs must fit in one page directory.
::static_assert::assert_eq!(MEMORY_SIZE / mem::PGTAB_SIZE <= PAGE_TABLE_LENGTH);

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
/// Upon success, `Ok(())`. Upon failure, an error is returned and the global state remains
/// uninitialized (atomics are not published).
///
/// # Notes
///
/// On x86, `kernel_cr3` equals `kernel_pd_paddr` (the page directory is the CR3 root).
///
/// This function pre-allocates a BSS page table for every PDE index in
/// `[0, MEMORY_SIZE)` that does not already have one. This covers all physical memory, so no
/// new PDEs are created at runtime. The kernel PD and CR3 atomics are published only after
/// pre-allocation succeeds, so other code never observes a partially-initialized identity map.
pub(crate) fn init(
    kernel_pd_paddr: PageDirectoryAddress,
    kernel_cr3: Cr3Register,
) -> Result<(), Error> { ... }

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
pub(crate) fn memcpy(dst: *mut u8, src: *const u8, size: usize) -> Result<(), Error> { ... }

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
pub(crate) fn memset(base: *mut u8, value: u8, size: usize) -> Result<(), Error> { ... }

///
/// # Description
///
/// Copies all present kernel identity-mapping PDEs from the kernel page directory into a target
/// page directory. This covers `[0, MEMORY_SIZE)` and ensures that the target PD (typically a
/// new user process PD) can access all kernel identity-mapped memory. Because all PDEs in this
/// range are pre-allocated at boot ([`init`]), this is a simple copy of already-present entries.
///
/// # Parameters
///
/// - `target_pd_paddr`: Physical address of the target page directory.
///
/// # Returns
///
/// Upon success, `Ok(())`. Upon failure, an error is returned.
///
/// # Notes
///
/// This function should be called once when constructing a new user address space.
///
pub(crate) fn sync_kernel_pdes(target_pd_paddr: PageDirectoryAddress) -> Result<(), Error> { ... }

/// RAII guard that restores the original CR3 value when dropped.
struct Cr3Guard(Cr3Register);

impl Drop for Cr3Guard {
    ///
    /// # Description
    ///
    /// Restores the previously active CR3 value saved in this guard.
    ///
    fn drop(&mut self) { ... }
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
fn with_kernel_address_space<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{ ... }

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
) -> Result<(PageAligned<PhysicalAddress>, usize), Error> { ... }

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
) -> Result<(), Error> { ... }

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
fn ensure_pt(pd: Table<PageDirectoryEntry>, pde_idx: TableIndex) -> Result<usize, Error> { ... }

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
) -> Result<(), Error> { ... }

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
pub(crate) fn identity_map_page(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> { ... }

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(feature = "test")]
pub(super) mod test {
    use super::{
        sync_kernel_pdes,
        KERNEL_PD_PADDR,
    };
    use crate::{
        hal::mem::PageDirectoryAddress,
        mm::VirtMemoryManager,
    };
    use ::arch::mem::{
        self,
        paging::{
            self,
            PageDirectoryEntry,
            Table,
            TableIndex,
        },
    };
    use ::config::kernel::MEMORY_SIZE;
    use ::core::sync::atomic::Ordering;

    ///
    /// # Description
    ///
    /// Verifies that [`super::init`] pre-allocates a page table for every PDE index in
    /// `[0, MEMORY_SIZE)`. Reads each PDE from the kernel page directory and
    /// asserts the present bit is set.
    ///
    fn test_init_preallocates_identity_map_pdes() -> bool { ... }

    ///
    /// # Description
    ///
    /// Allocates a zeroed kernel page, treats it as a page directory, calls
    /// [`sync_kernel_pdes`], and verifies that every present kernel PDE in
    /// `[0, MEMORY_SIZE)` was copied into the target PD with the same frame address.
    ///
    fn test_sync_kernel_pdes_copies_to_target() -> bool { ... }

    /// Runs all identity-mapping virtual memory tests.
    pub fn test() -> bool { ... }
}
