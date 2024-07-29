// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod kpage;
mod manager;
mod upage;
mod vmem;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    hal::{
        arch::x86::mem::mmu::{
            self,
            page_table::{
                PageTable,
                PageTableStorage,
            },
        },
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
    },
    klib,
};
use ::alloc::{
    collections::LinkedList,
    vec::Vec,
};
use ::core::cmp::Ordering;
use ::sys::{
    config,
    error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Exports
//==================================================================================================

pub use kpage::KernelPage;
pub use manager::VirtMemoryManager;
pub use vmem::Vmem;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// FIXME: this function is too long and complex.
pub fn init(
    mut virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    mut mmio_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<LinkedList<(PageTableAddress, PageTable)>, Error> {
    info!("booking virtual memory regions ...");

    let mut root_pagetables: LinkedList<(PageTableAddress, PageTable)> = LinkedList::new();

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
        assert!(region.size() <= mem::PGTAB_SIZE);

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
            let (page_table_addr, mut page_table): (PageTableAddress, PageTable) =
                if let Some(last) = root_pagetables.pop_back() {
                    let page_table_addr: PageTableAddress =
                        PageTableAddress::new(PageTableAligned::from_address(
                            VirtualAddress::new(klib::align_down(raw_vaddr, mmu::PGTAB_ALIGNMENT)),
                        )?);

                    match page_table_addr.cmp(&last.0) {
                        Ordering::Greater => {
                            root_pagetables.push_back(last);
                            let page_table: PageTable = PageTable::new(PageTableStorage::new());
                            let page_table_addr: PageTableAligned<VirtualAddress> =
                                PageTableAligned::from_address(VirtualAddress::new(
                                    klib::align_down(raw_vaddr, mmu::PGTAB_ALIGNMENT),
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
                    let page_table: PageTable = PageTable::new(PageTableStorage::new());
                    let page_table_addr: PageTableAligned<VirtualAddress> =
                        PageTableAligned::from_address(VirtualAddress::new(klib::align_down(
                            raw_vaddr,
                            mmu::PGTAB_ALIGNMENT,
                        )))?;
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
            if raw_vaddr == (config::kernel::MEMORY_SIZE - mem::PAGE_SIZE) {
                break;
            }
            raw_vaddr = raw_vaddr + mem::PAGE_SIZE;
            paddr = match region.typ() {
                MemoryRegionType::Mmio => {
                    let mmio_addr: VirtualAddress = region.start().into_inner();
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

            root_pagetables.push_back((page_table_addr, page_table));
        }
    }

    Ok(root_pagetables)
}
