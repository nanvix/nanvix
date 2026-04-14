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
pub(crate) mod virt;

#[cfg(feature = "hyperlight")]
pub(crate) use virt::phys_memcpy;

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
    hal::mem::{
        Address,
        FrameAddress,
        MemoryRegion,
        MemoryRegionType,
        PageAligned,
        PageTableAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
        VirtualAddress,
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
#[cfg(target_arch = "x86")]
::static_assert::assert_eq!(config::kernel::KPOOL_SIZE <= mem::PGTAB_SIZE);
// Ensure that the kernel stack size is multiple of a page size.
::static_assert::assert_eq!(config::kernel::KSTACK_SIZE.is_multiple_of(PAGE_ALIGNMENT as usize));
// Ensure that the kernel stack size is at least two pages (one guard page + one usable page).
::static_assert::assert_eq!(config::kernel::KSTACK_SIZE >= 2 * mem::PAGE_SIZE);
// Ensure that the kernel stack size fits in a single page table.
#[cfg(target_arch = "x86")]
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
// Only Reserved regions are processed — MMIO regions are handled separately
// in phase 2 via mmio_regions.
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
        if region.typ() == MemoryRegionType::Reserved {
            if PhysicalAddress::from_virtual_address(region.start()).is_ok() {
                match TruncatedMemoryRegion::from_virtual_memory_region(region.clone()) {
                    Ok(region) => physical_memory_regions.push_back(region),
                    // TODO: make memory regions a truncated list so round logic is handled when region is created.
                    Err(err) => panic!(
                        "failed to create physical memory region {:?} (error={:?})",
                        region, err
                    ),
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
) -> Result<Vmem, Error> {
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

    // Phase 1: map memory regions using BSS boot allocator.
    let (kernel_pages, kernel_page_tables): (
        LinkedList<KernelPage>,
        LinkedList<(PageTableAligned<VirtualAddress>, PageTableStorage)>,
    ) = (LinkedList::new(), virt::init(virtual_memory_regions)?);

    let mut vmem: Vmem = VirtMemoryManager::init(kernel_pages, kernel_page_tables, physman)?;

    // Phase 2: map MMIO regions using the kernel page pool allocator.
    // PTs for [0, MEMORY_SIZE) are pre-installed with empty PTEs, so MMIO pages
    // within that range will have their PTEs filled without conflict.
    for region in mmio_regions.iter() {
        info!("mapping mmio: {:?}", region);
        let mut raw_vaddr: usize = region.start().into_raw_value();
        let end: usize = raw_vaddr + (region.size() - 1);

        while raw_vaddr < end {
            let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?;
            let mmio_addr: VirtualAddress = VirtualAddress::from_raw_value(raw_vaddr);
            // FIXME: ensure safety here.
            let phys_addr: PhysicalAddress =
                unsafe { PhysicalAddress::from_mmio_address(mmio_addr)? };
            let frame: FrameAddress = FrameAddress::new(PageAligned::from_address(phys_addr)?);

            let page_table_allocator = || {
                let kpage: KernelPage = {
                    // SAFETY: the memory manager is initialized and access is synchronized.
                    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
                    mm.alloc_kpage(true)?
                };
                Ok(PageTableStorage::KernelPage(kpage))
            };

            vmem.map_mmio_page(frame, vaddr, page_table_allocator)?;

            match raw_vaddr.checked_add(mem::PAGE_SIZE) {
                Some(next) => raw_vaddr = next,
                None => break,
            };
        }
    }

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
                    Ok(PageTableStorage::KernelPage(kpage))
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
