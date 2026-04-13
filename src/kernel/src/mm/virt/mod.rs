// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod identity_map;
mod kpage;
mod manager;
mod page_table_allocator;
pub(crate) mod vmem;

#[cfg(feature = "hyperlight")]
pub(crate) use identity_map::memcpy;

//==================================================================================================
// Imports
//==================================================================================================

use self::page_table_allocator::PAGE_TABLE_ALLOCATOR;
use crate::hal::{
    arch::x86::mem::mmu::page_table::PageTable,
    mem::{
        AccessPermission,
        Address,
        FrameAddress,
        MemoryRegionType,
        PageAddress,
        PageAligned,
        PageTableAddress,
        PageTableAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
        VirtualAddress,
    },
};
use ::alloc::{
    collections::LinkedList,
    vec::Vec,
};
use ::arch::{
    mem,
    mem::{
        paging::{
            AccessedFlag,
            DirtyFlag,
            PageCacheDisableFlag,
            PageTableEntryFlags,
            PageWriteThroughFlag,
            PresentFlag,
            PteWord,
            ReadWriteFlag,
            UserSupervisorFlag,
        },
        PAGE_TABLE_LENGTH,
        PGTAB_ALIGNMENT,
    },
};
use ::core::{
    cmp::Ordering,
    ops::{
        Deref,
        DerefMut,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use kpage::KernelPage;
pub use manager::VirtMemoryManager;
pub use vmem::Vmem;

//==================================================================================================
// Structures and Enums
//==================================================================================================

pub enum PageTableStorage {
    /// Boot-time BSS-backed storage, allocated via `PAGE_TABLE_ALLOCATOR`.
    Bss(&'static mut [PteWord; PAGE_TABLE_LENGTH]),
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
    /// Scratch memory storage (always writable, identity-mapped).
    /// Used on Hyperlight to wrap existing page tables built by the host.
    #[allow(dead_code)]
    Scratch(*mut [u32; PAGE_TABLE_LENGTH]),
}

// SAFETY: Scratch pointers reference stable, process-global scratch memory.
unsafe impl Send for PageTableStorage {}

impl Deref for PageTableStorage {
    type Target = [PteWord];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss(entries) => entries.as_slice(),
            Self::KernelPage(page) => {
                let base: *const PteWord = page.base().into_raw_value() as *const PteWord;
                unsafe { core::slice::from_raw_parts(base, PAGE_TABLE_LENGTH) }
            },
            Self::Scratch(ptr) => unsafe { &**ptr },
        }
    }
}

impl DerefMut for PageTableStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Bss(entries) => entries.as_mut_slice(),
            Self::KernelPage(page) => {
                let base: *mut PteWord = page.base().into_raw_value() as *mut PteWord;
                unsafe { core::slice::from_raw_parts_mut(base, PAGE_TABLE_LENGTH) }
            },
            Self::Scratch(ptr) => unsafe { &mut **ptr },
        }
    }
}

pub enum PageDirectoryStorage {
    /// Boot-time BSS-backed storage, allocated via `PAGE_TABLE_ALLOCATOR`.
    Bss(&'static mut [PteWord; PAGE_TABLE_LENGTH]),
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
    /// Scratch memory storage (always writable, identity-mapped).
    /// Used on Hyperlight to wrap the existing page directory built by the host.
    #[allow(dead_code)]
    Scratch(*mut [u32; PAGE_TABLE_LENGTH]),
}

// SAFETY: Scratch pointers reference stable, process-global scratch memory.
unsafe impl Send for PageDirectoryStorage {}

impl PageDirectoryStorage {
    /// Wraps the existing Hyperlight-built page directory at the given CR3 physical address.
    /// The address must point to identity-mapped scratch memory.
    #[cfg(feature = "hyperlight")]
    #[allow(dead_code)]
    pub fn from_cr3(cr3: u32) -> Self {
        Self::Scratch(cr3 as *mut [u32; PAGE_TABLE_LENGTH])
    }
}


impl Deref for PageDirectoryStorage {
    type Target = [PteWord];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss(entries) => entries.as_slice(),
            Self::KernelPage(page) => {
                let base: *const PteWord = page.base().into_raw_value() as *const PteWord;
                unsafe { core::slice::from_raw_parts(base, PAGE_TABLE_LENGTH) }
            },
            Self::Scratch(ptr) => unsafe { &**ptr },
        }
    }
}

impl DerefMut for PageDirectoryStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Bss(entries) => entries.as_mut_slice(),
            Self::KernelPage(page) => {
                let base: *mut PteWord = page.base().into_raw_value() as *mut PteWord;
                unsafe { core::slice::from_raw_parts_mut(base, PAGE_TABLE_LENGTH) }
            },
            Self::Scratch(ptr) => unsafe { &mut **ptr },
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// FIXME: this function is too long and complex.
#[allow(dead_code)]
pub fn init(
    mut virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    mut mmio_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>, Error> {
    info!("booking virtual memory regions ...");

    let mut root_pagetables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)> =
        LinkedList::new();

    // Sort memory regions by start address.
    let mut regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>> = {
        virtual_memory_regions.append(&mut mmio_memory_regions);
        let mut regions: Vec<_> = virtual_memory_regions.into_iter().collect();
        regions.sort();
        regions.into_iter().collect()
    };

    // Identity map memory regions.
    while let Some(region) = regions.pop_front() {
        info!("booking: {:?}", region);

        let raw_vaddr: usize = region.start().into_raw_value();

        let mut paddr: FrameAddress = match region.typ() {
            MemoryRegionType::Mmio => {
                let mmio_addr: VirtualAddress = region.start().into_inner();
                let phys_addr: PhysicalAddress =
                    // FIXME: ensure safety here.
                    unsafe { PhysicalAddress::from_mmio_address(mmio_addr)? };
                let page_aligned_phys_addr: PageAligned<PhysicalAddress> =
                    PageAligned::from_address(phys_addr)?;
                FrameAddress::new(page_aligned_phys_addr)
            },
            _ => FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(
                raw_vaddr,
            )?)?),
        };

        let mut raw_vaddr: usize = raw_vaddr;
        let end: usize = raw_vaddr + (region.size() - 1);

        while raw_vaddr < end {
            let (page_table_addr, mut page_table): (PageTableAddress, PageTable<PageTableStorage>) =
                if let Some(last) = root_pagetables.pop_back() {
                    let page_table_addr: PageTableAddress =
                        PageTableAddress::new(PageTableAligned::from_address(
                            VirtualAddress::new(::sys::mm::align_down(raw_vaddr, PGTAB_ALIGNMENT)),
                        )?);

                    match page_table_addr.cmp(&last.0) {
                        Ordering::Greater => {
                            root_pagetables.push_back(last);
                            let pgtable_storage: PageTableStorage =
                                // SAFETY: called during single-threaded kernel init;
                                // BSS is zero-initialized, so assume_init_mut() is sound.
                                PageTableStorage::Bss(unsafe {
                                    PAGE_TABLE_ALLOCATOR
                                        .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
                                        .map_err(|e| {
                                            error!("page table allocation failed: {}", e);
                                            Error::new(
                                                ErrorCode::OutOfMemory,
                                                "BSS page table allocation failed",
                                            )
                                        })?
                                        .assume_init_mut()
                                });
                            let page_table: PageTable<PageTableStorage> =
                                PageTable::<PageTableStorage>::new(pgtable_storage);
                            let page_table_addr: PageTableAligned<VirtualAddress> =
                                PageTableAligned::from_address(VirtualAddress::new(
                                    ::sys::mm::align_down(raw_vaddr, PGTAB_ALIGNMENT),
                                ))?;
                            (PageTableAddress::new(page_table_addr), page_table)
                        },
                        Ordering::Equal => last,
                        Ordering::Less => {
                            let reason: &str = "overlapping memory regions";
                            error!("{}: {:#010x}", reason, raw_vaddr);
                            return Err(Error::new(ErrorCode::InvalidArgument, reason));
                        },
                    }
                } else {
                    trace!("creating new page table for {:#010x}", raw_vaddr);
                    let pgtable_storage: PageTableStorage =
                        // SAFETY: called during single-threaded kernel init;
                        // BSS is zero-initialized, so assume_init_mut() is sound.
                        PageTableStorage::Bss(unsafe {
                            PAGE_TABLE_ALLOCATOR
                                .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
                                .map_err(|e| {
                                    error!("page table allocation failed: {}", e);
                                    Error::new(
                                        ErrorCode::OutOfMemory,
                                        "BSS page table allocation failed",
                                    )
                                })?
                                .assume_init_mut()
                        });
                    let page_table: PageTable<PageTableStorage> =
                        PageTable::<PageTableStorage>::new(pgtable_storage);
                    let page_table_addr: PageTableAligned<VirtualAddress> =
                        PageTableAligned::from_address(VirtualAddress::new(
                            ::sys::mm::align_down(raw_vaddr, PGTAB_ALIGNMENT),
                        ))?;
                    (PageTableAddress::new(page_table_addr), page_table)
                };

            if region.typ() != MemoryRegionType::Mmio {
                // Bulk identity fill: map all pages in this page table at once.
                // FIXME: do not be so open about permissions and caching.
                let pgtab_base: usize = ::sys::mm::align_down(raw_vaddr, PGTAB_ALIGNMENT);
                let start_index: usize = (raw_vaddr - pgtab_base) / mem::PAGE_SIZE;
                let pgtab_remaining: usize = PAGE_TABLE_LENGTH - start_index;
                let region_remaining: usize = (end - raw_vaddr) / mem::PAGE_SIZE + 1;
                let memory_remaining: usize =
                    config::kernel::MEMORY_SIZE.saturating_sub(raw_vaddr) / mem::PAGE_SIZE;
                let count: usize = pgtab_remaining.min(region_remaining).min(memory_remaining);

                if count == 0 {
                    break;
                }

                let fill_count: usize = page_table
                    .fill(
                        start_index,
                        count,
                        FrameAddress::from_raw_value(raw_vaddr)?,
                        PageTableEntryFlags::new(
                            PresentFlag::Present,
                            ReadWriteFlag::ReadWrite,
                            UserSupervisorFlag::Supervisor,
                            PageWriteThroughFlag::WriteThrough,
                            PageCacheDisableFlag::CacheDisabled,
                            AccessedFlag::NotAccessed,
                            DirtyFlag::NotDirty,
                        ),
                        false,
                    )
                    .map_err(|(_count, e)| e)?;
                debug_assert!(fill_count == count, "fill_count ({fill_count}) != count ({count})");

                root_pagetables.push_back((page_table_addr, page_table));
                // NOTE: `count` is bounded by `memory_remaining`, so this cannot overflow.
                raw_vaddr += count
                    .checked_mul(mem::PAGE_SIZE)
                    .expect("count * PAGE_SIZE overflow");
                if raw_vaddr >= config::kernel::MEMORY_SIZE {
                    break;
                }
                paddr = FrameAddress::new(PageAligned::from_address(
                    PhysicalAddress::from_raw_value(raw_vaddr)?,
                )?);
            } else {
                // MMIO: per-page mapping with address translation.
                // FIXME: do not be so open about permissions and caching.
                page_table.map(
                    PageAddress::new(PageAligned::from_raw_value(raw_vaddr)?),
                    paddr,
                    true,
                    true,
                    false,
                    AccessPermission::RDWR,
                )?;
                root_pagetables.push_back((page_table_addr, page_table));
                if raw_vaddr == (config::kernel::MEMORY_SIZE - mem::PAGE_SIZE) {
                    break;
                }
                raw_vaddr += mem::PAGE_SIZE;
                paddr = {
                    let mmio_addr: VirtualAddress = VirtualAddress::new(raw_vaddr);
                    let phys_addr: PhysicalAddress =
                        // FIXME: ensure safety here.
                        unsafe { PhysicalAddress::from_mmio_address(mmio_addr)? };
                    FrameAddress::new(PageAligned::from_address(phys_addr)?)
                };
            }
        }
    }

    Ok(root_pagetables)
}

///
/// # Description
///
/// Reads the current CR3 register, walks the Hyperlight-built page directory,
/// and wraps the existing page tables in Scratch-backed storage. The PTs stay
/// in scratch memory (always writable) so the guest CoW page fault handler can
/// modify PTEs. The host rebuilds these PTs on each restore via
/// `update_scratch_bookkeeping()`.
///
/// # Returns
///
/// A tuple of:
/// - The Scratch-backed page directory storage (wrapping Hyperlight's PD in scratch).
/// - A list of `(PageTableAddress, PageTable<PageTableStorage>)` for every present PDE.
///
/// # Safety
///
/// Must only be called during single-threaded kernel init.
///
#[cfg(feature = "hyperlight")]
pub fn from_hyperlight_cr3() -> Result<
    (
        PageDirectoryStorage,
        LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ),
    Error,
> {
    let cr3: u32 = unsafe {
        let val: u32;
        core::arch::asm!("mov {0:e}, cr3", out(reg) val);
        val
    };

    let pd_ptr: *mut [u32; PAGE_TABLE_LENGTH] = cr3 as *mut [u32; PAGE_TABLE_LENGTH];
    let pd: &[u32; PAGE_TABLE_LENGTH] = unsafe { &*pd_ptr };

    let mut page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)> =
        LinkedList::new();

    for pdi in 0..1024u32 {
        let pde = pd[pdi as usize];
        if (pde & 1) == 0 {
            continue; // Not present.
        }

        let pt_phys = pde & 0xFFFFF000;
        let pt_ptr: *mut [u32; PAGE_TABLE_LENGTH] = pt_phys as *mut [u32; PAGE_TABLE_LENGTH];
        let pt: &[u32; PAGE_TABLE_LENGTH] = unsafe { &*pt_ptr };

        // Count mapped (present) pages in this PT.
        let nmapped = pt.iter().filter(|&&e| (e & 1) != 0).count();

        // Wrap the scratch-resident PT directly — no copy needed.
        // Scratch memory is always writable, so CoW PTE updates work.
        let storage = PageTableStorage::Scratch(pt_ptr);
        let page_table = PageTable::from_existing(storage, nmapped);

        let va = (pdi << 22) as usize;
        let pt_addr = PageTableAddress::new(PageTableAligned::from_address(
            VirtualAddress::new(va),
        )?);

        page_tables.push_back((pt_addr, page_table));
    }

    let pd_storage = PageDirectoryStorage::Scratch(pd_ptr);
    Ok((pd_storage, page_tables))
}
