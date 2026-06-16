// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Memory manager initialization.
//!
//! The kernel builds its own root virtual address space: identity-mapped page tables are created,
//! CR3 is loaded, and any virtual memory regions that lie outside physical memory (e.g., MMIO
//! pages above `MEMORY_SIZE`) are explicitly mapped.

use crate::hal::{
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
};
use ::alloc::{
    collections::LinkedList,
    vec::Vec,
};
use ::arch::mem;
use ::bitmap::Bitmap;
use ::sys::error::Error;

use super::{
    phys,
    virt,
    KernelPage,
    PageTableStorage,
    PhysMemRegion,
    VirtMemRegion,
    VirtMemoryManager,
    Vmem,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Splits memory regions into virtual and physical (kernel-built VAS path).
///
/// All GVAs are identity-mapped, so virtual addresses can be directly converted to physical
/// addresses without translation.
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
                match TruncatedMemoryRegion::from_virtual_memory_region(region.clone()) {
                    Ok(region) => physical_memory_regions.push_back(region),
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

///
/// # Description
///
/// Initializes the memory manager (kernel-built root VAS path).
///
/// # Parameters
///
/// - `memory_regions`: Memory regions.
/// - `mmio_regions`: MMIO regions.
/// - `physical_memory_layout`: Physical memory layout bitmap.
///
/// # Returns
///
/// Upon success, the root virtual memory manager is returned. Upon failure, an error is returned
/// instead.
///
pub fn init(
    memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
    mmio_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    physical_memory_layout: Bitmap,
) -> Result<Vmem, Error> {
    info!("initializing the memory manager ...");

    type VirtMemRegions = LinkedList<TruncatedMemoryRegion<VirtualAddress>>;
    type PhysMemRegions = LinkedList<TruncatedMemoryRegion<PhysicalAddress>>;

    let (mut other_virtual_memory_regions, virtual_memory_regions, physical_memory_regions): (
        VirtMemRegions,
        VirtMemRegions,
        PhysMemRegions,
    ) = parse_memory_regions(memory_regions)?;

    phys::init(physical_memory_regions, &mmio_regions, physical_memory_layout)?;

    #[cfg(feature = "test")]
    phys::test();

    // Build identity-mapped page tables and create the root address space.
    let (kernel_pages, kernel_page_tables): (
        LinkedList<KernelPage>,
        LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) = (LinkedList::new(), virt::init(virtual_memory_regions, mmio_regions)?);

    let mut vmem: Vmem = VirtMemoryManager::init(kernel_pages, kernel_page_tables)?;

    #[cfg(feature = "test")]
    virt::test();

    // Map virtual memory regions that lie outside the physical memory.
    while let Some(region) = other_virtual_memory_regions.pop_front() {
        info!("mapping: {:?}", region);
        let mut vaddr: PageAligned<VirtualAddress> = region.start();
        let end: VirtualAddress =
            VirtualAddress::new(region.start().into_raw_value() + (region.size() - 1));

        {
            while vaddr.into_inner() < end {
                let kpage: KernelPage = {
                    // SAFETY: the memory manager is initialized and access is synchronized.
                    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
                    mm.alloc_kpage(false)?
                };

                vmem.map_kpage(kpage, vaddr)?;

                match vaddr.into_raw_value().checked_add(mem::PAGE_SIZE) {
                    Some(raw_addr) => vaddr = PageAligned::from_raw_value(raw_addr)?,
                    None => break,
                };
            }
        }
    }

    Ok(vmem)
}
