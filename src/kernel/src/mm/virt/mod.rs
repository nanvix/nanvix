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
    arch::x86::mem::mmu::{
        self,
        page_table::PageTable,
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
};
use ::alloc::{
    boxed::Box,
    collections::LinkedList,
    vec::Vec,
};
use ::core::{
    cmp::Ordering,
    ops::{
        Deref,
        DerefMut,
    },
};
use ::sys::{
    arch::mem,
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
// Structures and Enums
//==================================================================================================

pub enum PageTableStorage {
    Heap(Box<[u32; mem::PAGE_SIZE / core::mem::size_of::<u32>()]>),
    KernelPage(KernelPage),
}

impl Deref for PageTableStorage {
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Heap(entries) => entries.deref(),
            Self::KernelPage(page) => {
                let base: *const u32 = page.base().into_raw_value() as *const u32;
                unsafe {
                    core::slice::from_raw_parts(base, mem::PAGE_SIZE / core::mem::size_of::<u32>())
                }
            },
        }
    }
}

impl DerefMut for PageTableStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Heap(entries) => entries.deref_mut(),
            Self::KernelPage(page) => {
                let base: *mut u32 = page.base().into_raw_value() as *mut u32;
                unsafe {
                    core::slice::from_raw_parts_mut(
                        base,
                        mem::PAGE_SIZE / core::mem::size_of::<u32>(),
                    )
                }
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
                    let page_table_addr: PageTableAddress = PageTableAddress::new(
                        PageTableAligned::from_address(VirtualAddress::new(
                            ::sys::mm::align_down(raw_vaddr, mmu::PGTAB_ALIGNMENT),
                        ))?,
                    );

                    match page_table_addr.cmp(&last.0) {
                        Ordering::Greater => {
                            root_pagetables.push_back(last);
                            let pgtable_storage: PageTableStorage = PageTableStorage::Heap(
                                Box::new([0; mem::PAGE_SIZE / core::mem::size_of::<u32>()]),
                            );
                            let page_table: PageTable<PageTableStorage> =
                                PageTable::<PageTableStorage>::new(pgtable_storage);
                            let page_table_addr: PageTableAligned<VirtualAddress> =
                                PageTableAligned::from_address(VirtualAddress::new(
                                    ::sys::mm::align_down(raw_vaddr, mmu::PGTAB_ALIGNMENT),
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
                    let pgtable_storage: PageTableStorage = PageTableStorage::Heap(Box::new(
                        [0; mem::PAGE_SIZE / core::mem::size_of::<u32>()],
                    ));
                    let page_table: PageTable<PageTableStorage> =
                        PageTable::<PageTableStorage>::new(pgtable_storage);
                    let page_table_addr: PageTableAligned<VirtualAddress> =
                        PageTableAligned::from_address(VirtualAddress::new(
                            ::sys::mm::align_down(raw_vaddr, mmu::PGTAB_ALIGNMENT),
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
            if raw_vaddr == (config::kernel::MEMORY_SIZE - mem::PAGE_SIZE) {
                break;
            }
            raw_vaddr += mem::PAGE_SIZE;
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
