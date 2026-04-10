// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod kpage;
mod manager;
mod vmem;

//==================================================================================================
// Imports
//==================================================================================================

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
            PageTableEntry,
            PageTableEntryFlags,
            PageWriteThroughFlag,
            PresentFlag,
            PteWord,
            ReadWriteFlag,
            UserSupervisorFlag,
        },
        PGTAB_ALIGNMENT,
    },
};
use ::core::{
    cell::UnsafeCell,
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
// Constants
//==================================================================================================

// Number of page tables needed to identity-map all physical memory plus MMIO regions at boot.
::static_assert::assert_eq!(config::kernel::MEMORY_SIZE.is_multiple_of(mem::PGTAB_SIZE));

const NUM_BOOT_PAGE_TABLES: usize =
    config::kernel::MEMORY_SIZE / mem::PGTAB_SIZE + config::platform::NUM_MMIO_BOOT_PAGE_TABLES;

/// Total number of boot page-table-sized slots: page tables + 1 slot for the root page directory.
pub(crate) const NUM_BOOT_SLOTS: usize = NUM_BOOT_PAGE_TABLES + 1;

/// Number of u32 entries in a single page table / page directory (4096 / 4 = 1024).
const PAGE_TABLE_LENGTH: usize = mem::PAGE_SIZE / PageTableEntry::SIZE;

//==================================================================================================
// Boot Page Table BSS Storage
//==================================================================================================

/// Page-aligned BSS storage for boot page tables and the root page directory.
#[repr(align(4096))]
struct BootPageTableStorage {
    tables: [[PteWord; PAGE_TABLE_LENGTH]; NUM_BOOT_SLOTS],
}

::static_assert::assert_eq_align!(BootPageTableStorage, mem::PAGE_SIZE);

struct BootPageTableStorageWrapper(UnsafeCell<BootPageTableStorage>);

// SAFETY: Only accessed during single-threaded kernel init via `alloc_boot_slot()`.
unsafe impl Sync for BootPageTableStorageWrapper {}

static BOOT_STORAGE: BootPageTableStorageWrapper =
    BootPageTableStorageWrapper(UnsafeCell::new(BootPageTableStorage {
        tables: [[0; PAGE_TABLE_LENGTH]; NUM_BOOT_SLOTS],
    }));

/// Next available slot in the boot storage bump allocator.
struct BootSlotNextWrapper(UnsafeCell<usize>);

// SAFETY: Only accessed during single-threaded kernel init via `alloc_boot_slot()`.
unsafe impl Sync for BootSlotNextWrapper {}

static BOOT_SLOT_NEXT: BootSlotNextWrapper = BootSlotNextWrapper(UnsafeCell::new(0));

/// Flag that marks the boot allocator as sealed (no further allocations allowed).
struct BootSealedWrapper(UnsafeCell<bool>);

// SAFETY: Only accessed during single-threaded kernel init.
unsafe impl Sync for BootSealedWrapper {}

static BOOT_SEALED: BootSealedWrapper = BootSealedWrapper(UnsafeCell::new(false));

///
/// # Description
///
/// Seals the boot page-table bump allocator, preventing further allocations.
///
/// This should be called once the kernel page pool is available to catch accidental late uses.
///
/// # Safety
///
/// This function mutates global state and must only be called during single-threaded kernel init.
///
pub(crate) unsafe fn seal_boot_allocator() {
    *BOOT_SEALED.0.get() = true;
}

///
/// # Description
///
/// Allocates the next page-aligned boot slot from BSS storage.
///
/// This is a simple bump allocator used during early kernel initialization before the kernel page
/// pool is available. Each slot is exactly one page (4096 bytes) of `[PteWord; PAGE_TABLE_LENGTH]`.
///
/// # Panics
///
/// Panics if all boot slots have been exhausted or if the allocator has been sealed.
///
/// # Safety
///
/// This function mutates global state and must only be called during single-threaded kernel init.
///
pub(crate) unsafe fn alloc_boot_slot() -> &'static mut [PteWord; PAGE_TABLE_LENGTH] {
    assert!(!*BOOT_SEALED.0.get(), "boot allocator is sealed; allocations are no longer allowed");
    let next: *mut usize = BOOT_SLOT_NEXT.0.get();
    let idx: usize = *next;
    assert!(idx < NUM_BOOT_SLOTS, "boot page table storage exhausted");
    *next += 1;
    &mut (*BOOT_STORAGE.0.get()).tables[idx]
}

//==================================================================================================
// Structures and Enums
//==================================================================================================

pub enum PageTableStorage {
    /// Boot-time BSS-backed storage, allocated via `alloc_boot_slot()`.
    Bss(&'static mut [PteWord; PAGE_TABLE_LENGTH]),
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
}

impl Deref for PageTableStorage {
    type Target = [PteWord];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss(entries) => entries.as_slice(),
            Self::KernelPage(page) => {
                let base: *const PteWord = page.base().into_raw_value() as *const PteWord;
                unsafe { core::slice::from_raw_parts(base, PAGE_TABLE_LENGTH) }
            },
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
        }
    }
}

pub enum PageDirectoryStorage {
    /// Boot-time BSS-backed storage, allocated via `alloc_boot_slot()`.
    Bss(&'static mut [PteWord; PAGE_TABLE_LENGTH]),
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
}

impl PageDirectoryStorage {
    /// Allocates a page directory from BSS boot storage (used for the root page directory during
    /// early init, before the kernel page pool is available).
    ///
    /// # Safety
    ///
    /// This function must only be called during early single-threaded kernel init.
    pub unsafe fn new_bss() -> Self {
        Self::Bss(alloc_boot_slot())
    }

    /// Creates a page directory backed by a kernel page from the kernel page pool (used at runtime
    /// for new process address spaces).
    pub fn new_from_kpage(kpage: KernelPage) -> Self {
        Self::KernelPage(kpage)
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
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// FIXME: this function is too long and complex.
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
                                // SAFETY: called during single-threaded kernel init.
                                PageTableStorage::Bss(unsafe { alloc_boot_slot() });
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
                        // SAFETY: called during single-threaded kernel init.
                        PageTableStorage::Bss(unsafe { alloc_boot_slot() });
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
