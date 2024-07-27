// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
pub use virt::{
    KernelPage,
    VirtMemoryManager,
    Vmem,
    USER_BASE,
    USER_STACK_TOP,
};

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    error::Error,
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
use alloc::{
    collections::LinkedList,
    vec::Vec,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Splits memory regions into virtual and physical.
fn parse_memory_regions(
    memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
) -> Result<
    (
        LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
        LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
        LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
    ),
    Error,
> {
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
                    if let Ok(region) =
                        TruncatedMemoryRegion::from_virtual_memory_region(region.clone())
                    {
                        physical_memory_regions.push_back(region);
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

    let (mut other_virtual_memory_regions, virtual_memory_regions, physical_memory_regions): (
        LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
        LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
        LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
    ) = parse_memory_regions(memory_regions)?;

    let physman: PhysMemoryManager = phys::init(
        TruncatedMemoryRegion::from_virtual_memory_region(kimage.kpool())?,
        physical_memory_regions,
        &mmio_regions,
    )?;

    // FIXME: the initial list of kernel pages should be spit out by the initialization.
    let (kernel_pages, kernel_page_tables): (
        LinkedList<KernelPage>,
        LinkedList<(PageTableAddress, PageTable)>,
    ) = (LinkedList::new(), virt::init(virtual_memory_regions, mmio_regions)?);

    let (mut vmem, mut mm): (Vmem, VirtMemoryManager) =
        VirtMemoryManager::new(kernel_pages, kernel_page_tables, physman)?;

    // Map virtual memory regions that lie outside the physical memory.
    while let Some(region) = other_virtual_memory_regions.pop_front() {
        info!("mapping: {:?}", region);
        let mut vaddr: PageAligned<VirtualAddress> = region.start();
        let end: VirtualAddress =
            VirtualAddress::new(region.start().into_raw_value() + (region.size() - 1));

        while vaddr.into_inner() < end {
            let kpage: KernelPage = mm.alloc_kpage(false)?;
            vmem.map_kpage(kpage, vaddr)?;

            match vaddr.into_raw_value().checked_add(mem::PAGE_SIZE) {
                Some(raw_addr) => vaddr = PageAligned::from_raw_value(raw_addr)?,
                None => break,
            };
        }
    }

    Ok((vmem, mm))
}

// Returns the user base stack address.
pub fn user_stack_top() -> PageAligned<VirtualAddress> {
    PageAligned::from_address(USER_STACK_TOP).unwrap()
}
