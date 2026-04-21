// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(clippy::absurd_extreme_comparisons)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod elf;
mod phys;
mod virt;

#[cfg(feature = "hyperlight")]
pub(crate) use virt::memcpy;

//==================================================================================================
// Exports
//==================================================================================================

pub mod kheap;
use ::arch::mem::{
    PAGE_ALIGNMENT,
    PGTAB_ALIGNMENT,
};
pub use virt::{
    KernelPage,
    PageTableStorage,
    VirtMemoryManager,
    Vmem,
};
pub mod kstack;
pub mod ustack;

#[cfg(feature = "smp")]
pub mod kredzone;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::Bitmap,
    hal::{
        arch::x86::mem::mmu::page_table::PageTable,
        mem::{
            Address,
            MemoryRegion,
            MemoryRegionType,
            PageAligned,
            PageTableAddress,
            PhysicalAddress,
            TruncatedMemoryRegion,
            VirtualAddress,
        },
    },
    kimage::KernelImage,
};
use ::alloc::{
    collections::LinkedList,
    vec::Vec,
};
use ::arch::mem;
use ::core::panic;
use ::sparse_bitmap::SparseBitmap;
use ::sys::error::Error;

//==================================================================================================
// Static Assertions
//==================================================================================================

// Ensure that the kernel pool size is multiple of a page size.
::static_assert::assert_eq!(config::kernel::KPOOL_SIZE.is_multiple_of(PAGE_ALIGNMENT as usize));
// Ensure that the kernel pool size fits in a single page table.
::static_assert::assert_eq!(config::kernel::KPOOL_SIZE <= mem::PGTAB_SIZE);
// Ensure that the kernel stack size is multiple of a page size.
::static_assert::assert_eq!(config::kernel::KSTACK_SIZE.is_multiple_of(PAGE_ALIGNMENT as usize));
// Ensure that the kernel stack size is at least two pages (one guard page + one usable page).
::static_assert::assert_eq!(config::kernel::KSTACK_SIZE >= 2 * mem::PAGE_SIZE);
// Ensure that the kernel stack size fits in a single page table.
::static_assert::assert_eq!(config::kernel::KSTACK_SIZE <= mem::PGTAB_SIZE);
// Ensure that the kernel base address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::KERNEL_BASE_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the kernel base address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::KERNEL_BASE_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
// Ensure that the kernel end address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::KERNEL_END_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the kernel end address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::KERNEL_END_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
// Ensure that the user base address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_BASE_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the user base address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_BASE_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
// Ensure that the user end address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_END_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the user end address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_END_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
// Ensure that the user stack base address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_STACK_BASE_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the user stack base address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_STACK_BASE_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
// Ensure that the mmap base address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_MMAP_BASE_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the mmap base address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_MMAP_BASE_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
// Ensure that the mmap end address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_MMAP_END_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the mmap end address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_MMAP_END_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
// Ensure that the user and kernel address spaces do not overlap.
::static_assert::assert_eq!(
    config::memory_layout::USER_BASE_RAW < config::memory_layout::USER_END_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::KERNEL_BASE_RAW < config::memory_layout::KERNEL_END_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_BASE_RAW >= config::memory_layout::KERNEL_END_RAW
);
// Ensure that the kernel pool lies within the kernel base and end addresses.
::static_assert::assert_eq!(
    config::memory_layout::KPOOL_BASE_RAW >= config::memory_layout::KERNEL_BASE_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::KPOOL_BASE_RAW + config::kernel::KPOOL_SIZE
        < config::memory_layout::KERNEL_END_RAW
);
// Ensure that the mmap region lies within the user base and end addresses.
::static_assert::assert_eq!(
    config::memory_layout::USER_MMAP_BASE_RAW >= config::memory_layout::USER_BASE_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_MMAP_END_RAW > config::memory_layout::USER_MMAP_BASE_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_MMAP_END_RAW < config::memory_layout::USER_END_RAW
);
// Ensure that the user stack lies within the user base and end addresses.
::static_assert::assert_eq!(
    config::memory_layout::USER_STACK_BASE_RAW <= config::memory_layout::USER_END_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_STACK_TOP_RAW >= config::memory_layout::USER_BASE_RAW
);
// Ensure that the minimum stack size is a multiple of a page size.
::static_assert::assert_eq!(
    config::memory_layout::USER_STACK_MIN_SIZE.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the minimum stack size does not exceed the total stack size.
::static_assert::assert_eq!(
    config::memory_layout::USER_STACK_MIN_SIZE <= config::memory_layout::USER_STACK_SIZE
);
// Ensure that the thread stack size is at least the minimum stack size.
::static_assert::assert_eq!(
    config::memory_layout::USER_THREAD_STACK_SIZE >= config::memory_layout::USER_STACK_MIN_SIZE
);

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Splits memory regions into virtual and physical.
type VirtMemRegion = LinkedList<TruncatedMemoryRegion<VirtualAddress>>;
type PhysMemRegion = LinkedList<TruncatedMemoryRegion<PhysicalAddress>>;

fn parse_memory_regions(
    memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
) -> Result<(VirtMemRegion, VirtMemRegion, PhysMemRegion), Error> {
    let mut memory_regions: LinkedList<MemoryRegion<VirtualAddress>> = {
        let mut memory_regions: Vec<_> = memory_regions.into_iter().collect();
        memory_regions.sort();
        memory_regions.into_iter().collect()
    };

    let mut virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>> =
        LinkedList::new();
    let mut other_virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>> =
        LinkedList::new();
    let mut physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>> =
        LinkedList::new();

    while let Some(region) = memory_regions.pop_front() {
        if region.typ() == MemoryRegionType::Reserved || region.typ() == MemoryRegionType::Mmio {
            if PhysicalAddress::from_virtual_address(region.start()).is_ok() {
                if region.typ() != MemoryRegionType::Usable {
                    match TruncatedMemoryRegion::from_virtual_memory_region(region.clone()) {
                        Ok(region) => physical_memory_regions.push_back(region),
                        // TODO: make memory regions a truncated list so round logic is handled when region is created.
                        Err(err) => panic!(
                            "failed to create physical memory region {:?} (error={:?})",
                            region, err
                        ),
                    }
                }
                virtual_memory_regions
                    .push_back(TruncatedMemoryRegion::from_memory_region(region)?);
            } else {
                other_virtual_memory_regions
                    .push_back(TruncatedMemoryRegion::from_memory_region(region)?);
            }
        }
    }

    Ok((other_virtual_memory_regions, virtual_memory_regions, physical_memory_regions))
}

///
/// # Description
///
/// Initializes the memory manager.
///
/// # Parameters
///
/// - `kimage`: Kernel image.
/// - `memory_regions`: Memory regions.
/// - `mmio_regions`: MMIO regions.
/// - `physical_memory_layout`: Physical memory layout bitmap.
/// - `kpool_bitmap`: Statically-allocated bitmap for the kernel page pool.
///
/// # Returns
///
/// Upon success, the root virtual memory manager is returned. Upon failure, an error is returned
/// instead.
///
pub fn init(
    kimage: &KernelImage,
    memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
    mmio_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    physical_memory_layout: SparseBitmap,
    kpool_bitmap: Bitmap,
) -> Result<Vmem, Error> {
    info!("initializing the memory manager ...");

    type VirtMemRegions = LinkedList<TruncatedMemoryRegion<VirtualAddress>>;
    type PhysMemRegions = LinkedList<TruncatedMemoryRegion<PhysicalAddress>>;

    let kpool_base: PageAligned<PhysicalAddress> =
        PageAligned::<PhysicalAddress>::from_raw_value(kimage.kpool().start().into_raw_value())?;

    let (mut other_virtual_memory_regions, virtual_memory_regions, physical_memory_regions): (
        VirtMemRegions,
        VirtMemRegions,
        PhysMemRegions,
    ) = parse_memory_regions(memory_regions)?;

    phys::init(
        kpool_base,
        physical_memory_regions,
        &mmio_regions,
        physical_memory_layout,
        kpool_bitmap,
    )?;

    #[cfg(feature = "test")]
    phys::test();

    // FIXME: the initial list of kernel pages should be spit out by the initialization.
    let (kernel_pages, kernel_page_tables): (
        LinkedList<KernelPage>,
        LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) = (LinkedList::new(), virt::init(virtual_memory_regions, mmio_regions)?);

    let mut vmem: Vmem = VirtMemoryManager::init(kernel_pages, kernel_page_tables)?;

    // Map virtual memory regions that lie outside the physical memory.
    while let Some(region) = other_virtual_memory_regions.pop_front() {
        info!("mapping: {:?}", region);
        let mut vaddr: PageAligned<VirtualAddress> = region.start();
        let end: VirtualAddress =
            VirtualAddress::new(region.start().into_raw_value() + (region.size() - 1));

        {
            while vaddr.into_inner() < end {
                // NOTE: each `VirtMemoryManager::get_mut()` borrow is scoped to its own inner
                // block so that the mutable reference is dropped before any subsequent borrow,
                // including the borrow that may occur when `page_table_allocator` is invoked
                // inside `map_kpage`.
                let kpage: KernelPage = {
                    // SAFETY: the memory manager is initialized and access is synchronized.
                    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
                    mm.alloc_kpage(false)?
                };

                let page_table_allocator = || {
                    let kpage: KernelPage = {
                        // SAFETY: the memory manager is initialized and access is synchronized.
                        let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
                        mm.alloc_kpage(true)?
                    };
                    let pgtable_storage: PageTableStorage = PageTableStorage::KernelPage(kpage);
                    let page_table: PageTable<PageTableStorage> = PageTable::new(pgtable_storage);
                    Ok(page_table)
                };

                vmem.map_kpage(kpage, vaddr, page_table_allocator)?;

                match vaddr.into_raw_value().checked_add(mem::PAGE_SIZE) {
                    Some(raw_addr) => vaddr = PageAligned::from_raw_value(raw_addr)?,
                    None => break,
                };
            }
        }
    }

    Ok(vmem)
}
