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

//==================================================================================================
// Exports
//==================================================================================================

pub mod kheap;
use ::alloc::boxed::Box;
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
    mm::phys::PhysMemoryManager,
};
use ::alloc::{
    collections::LinkedList,
    vec::Vec,
};
use ::arch::mem;
use ::core::panic;
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
// Ensure that the kernel stack size is at least one page.
::static_assert::assert_eq!(config::kernel::KSTACK_SIZE >= mem::PAGE_SIZE);
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
// Ensure that the user heap base address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_HEAP_BASE_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the user heap base address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_HEAP_BASE_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
//Ensure that the user libraries base address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_LIBS_BASE_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the user libraries base address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_LIBS_BASE_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
);
// Ensure that the user libraries end address is aligned to a page boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_LIBS_END_RAW.is_multiple_of(PAGE_ALIGNMENT as usize)
);
// Ensure that the user libraries end address is aligned to a page table boundary.
::static_assert::assert_eq!(
    config::memory_layout::USER_LIBS_END_RAW.is_multiple_of(PGTAB_ALIGNMENT as usize)
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
// Ensure that the user heap lies within the user base and end addresses.
::static_assert::assert_eq!(
    config::memory_layout::USER_HEAP_BASE_RAW >= config::memory_layout::USER_BASE_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_HEAP_BASE_RAW + config::memory_layout::USER_HEAP_SIZE
        < config::memory_layout::USER_END_RAW
);
// Ensure that the user stack lies within the user base and end addresses.
::static_assert::assert_eq!(
    config::memory_layout::USER_STACK_BASE_RAW <= config::memory_layout::USER_END_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_STACK_TOP_RAW >= config::memory_layout::USER_BASE_RAW
);
// Ensure that the user libraries base address lies within the user base and end addresses.
::static_assert::assert_eq!(
    config::memory_layout::USER_LIBS_BASE_RAW >= config::memory_layout::USER_BASE_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_LIBS_BASE_RAW < config::memory_layout::USER_END_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_LIBS_END_RAW > config::memory_layout::USER_LIBS_BASE_RAW
);
::static_assert::assert_eq!(
    config::memory_layout::USER_LIBS_END_RAW < config::memory_layout::USER_END_RAW
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

/// Initializes the memory manager.
pub fn init(
    kimage: &KernelImage,
    memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
    mmio_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<(Vmem, VirtMemoryManager), Error> {
    info!("initializing the memory manager ...");

    type VirtMemRegions = LinkedList<TruncatedMemoryRegion<VirtualAddress>>;
    type PhysMemRegions = LinkedList<TruncatedMemoryRegion<PhysicalAddress>>;

    let (mut other_virtual_memory_regions, virtual_memory_regions, physical_memory_regions): (
        VirtMemRegions,
        VirtMemRegions,
        PhysMemRegions,
    ) = parse_memory_regions(memory_regions)?;

    let physman: PhysMemoryManager = phys::init(
        TruncatedMemoryRegion::from_virtual_memory_region(kimage.kpool())?,
        physical_memory_regions,
        &mmio_regions,
    )?;

    // FIXME: the initial list of kernel pages should be spit out by the initialization.
    let (kernel_pages, kernel_page_tables): (
        LinkedList<KernelPage>,
        LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) = (LinkedList::new(), virt::init(virtual_memory_regions, mmio_regions)?);

    let (mut vmem, mut mm): (Vmem, VirtMemoryManager) =
        VirtMemoryManager::init(kernel_pages, kernel_page_tables, physman)?;

    // Map virtual memory regions that lie outside the physical memory.
    while let Some(region) = other_virtual_memory_regions.pop_front() {
        info!("mapping: {:?}", region);
        let mut vaddr: PageAligned<VirtualAddress> = region.start();
        let end: VirtualAddress =
            VirtualAddress::new(region.start().into_raw_value() + (region.size() - 1));

        while vaddr.into_inner() < end {
            let kpage: KernelPage = mm.alloc_kpage(false)?;

            let page_table_allocator = || {
                let pgtable_storage: PageTableStorage = PageTableStorage::Heap(Box::new(
                    [0; mem::PAGE_SIZE / core::mem::size_of::<u32>()],
                ));
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

    Ok((vmem, mm))
}
