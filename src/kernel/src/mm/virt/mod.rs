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
        paging::PageTableEntry,
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
    tables: [[u32; PAGE_TABLE_LENGTH]; NUM_BOOT_SLOTS],
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
/// pool is available. Each slot is exactly one page (4096 bytes) of `[u32; 1024]`.
///
/// # Panics
///
/// Panics if all boot slots have been exhausted or if the allocator has been sealed.
///
/// # Safety
///
/// This function mutates global state and must only be called during single-threaded kernel init.
///
pub(crate) unsafe fn alloc_boot_slot() -> &'static mut [u32; PAGE_TABLE_LENGTH] {
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
    Bss(&'static mut [u32; PAGE_TABLE_LENGTH]),
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
}

impl Deref for PageTableStorage {
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss(entries) => entries.as_slice(),
            Self::KernelPage(page) => {
                let base: *const u32 = page.base().into_raw_value() as *const u32;
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
                let base: *mut u32 = page.base().into_raw_value() as *mut u32;
                unsafe { core::slice::from_raw_parts_mut(base, PAGE_TABLE_LENGTH) }
            },
        }
    }
}

pub enum PageDirectoryStorage {
    /// Boot-time BSS-backed storage, allocated via `alloc_boot_slot()`.
    Bss(&'static mut [u32; PAGE_TABLE_LENGTH]),
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
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss(entries) => entries.as_slice(),
            Self::KernelPage(page) => {
                let base: *const u32 = page.base().into_raw_value() as *const u32;
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
                let base: *mut u32 = page.base().into_raw_value() as *mut u32;
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
            paddr = match region.typ() {
                MemoryRegionType::Mmio => {
                    let mmio_addr: VirtualAddress = VirtualAddress::new(raw_vaddr);
                    let phys_addr: PhysicalAddress =
                    // FIXME: ensure safety here.
                    unsafe { PhysicalAddress::from_mmio_address(mmio_addr)? };
                    let page_aligned_phys_addr: PageAligned<PhysicalAddress> =
                        PageAligned::from_address(phys_addr)?;
                    FrameAddress::new(page_aligned_phys_addr)
                },
                _ => FrameAddress::new(PageAligned::from_address(
                    PhysicalAddress::from_raw_value(raw_vaddr)?,
                )?),
            };
        }
    }

    Ok(root_pagetables)
}
