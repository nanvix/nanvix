// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Boot-Time Virtual Memory Initialization
//!
//! This module contains the `init` function that builds the root page tables during early kernel
//! boot on platforms that do **not** inherit a pre-built virtual address space from the host.
//!

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    page_table_allocator::PAGE_TABLE_ALLOCATOR,
    PageTableStorage,
};
use crate::hal::{
    arch::native::mem::mmu::page_table::PageTable,
    mem::{
        AccessPermission,
        Address,
        FrameAddress,
        MemoryRegionType,
        MmioCachePolicy,
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
use ::core::cmp::Ordering;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// FIXME: this function is too long and complex.
pub fn init(
    mut virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    mut mmio_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>, Error> {
    info!("booking virtual memory regions ...");

    // Last valid physical address (inclusive).
    let max_phys_addr: usize = PhysicalAddress::max_addr();

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
                    unsafe { PhysicalAddress::from_mmio_address(mmio_addr) };
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
                let pgtab_base: usize = ::sys::mm::align_down(raw_vaddr, PGTAB_ALIGNMENT);
                let start_index: usize = (raw_vaddr - pgtab_base) / mem::PAGE_SIZE;
                let pgtab_remaining: usize = PAGE_TABLE_LENGTH - start_index;
                let region_remaining: usize = (end - raw_vaddr) / mem::PAGE_SIZE + 1;
                let memory_remaining: usize = if raw_vaddr > max_phys_addr {
                    0
                } else {
                    (max_phys_addr - raw_vaddr) / mem::PAGE_SIZE + 1
                };
                let count: usize = pgtab_remaining.min(region_remaining).min(memory_remaining);

                if count == 0 {
                    break;
                }

                let rw_flag: ReadWriteFlag = if region.perm() == AccessPermission::RDWR {
                    ReadWriteFlag::ReadWrite
                } else {
                    ReadWriteFlag::ReadOnly
                };

                let fill_count: usize = page_table
                    .fill(
                        start_index,
                        count,
                        FrameAddress::from_raw_value(raw_vaddr)?,
                        PageTableEntryFlags::new(
                            PresentFlag::Present,
                            rw_flag,
                            UserSupervisorFlag::Supervisor,
                            PageWriteThroughFlag::NotWriteThrough,
                            PageCacheDisableFlag::CacheEnabled,
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
                if raw_vaddr > max_phys_addr || raw_vaddr >= end {
                    break;
                }
                paddr = FrameAddress::new(PageAligned::from_address(
                    PhysicalAddress::from_raw_value(raw_vaddr)?,
                )?);
            } else {
                // MMIO: per-page mapping with address translation.
                let cache_policy: MmioCachePolicy = region
                    .cache_policy()
                    .unwrap_or(MmioCachePolicy::UNCACHEABLE);
                page_table.map(
                    PageAddress::new(PageAligned::from_raw_value(raw_vaddr)?),
                    paddr,
                    true,
                    cache_policy.write_through(),
                    cache_policy.cache_enabled(),
                    region.perm(),
                )?;
                root_pagetables.push_back((page_table_addr, page_table));
                if raw_vaddr == max_phys_addr - (mem::PAGE_SIZE - 1) {
                    break;
                }
                raw_vaddr += mem::PAGE_SIZE;
                paddr = {
                    let mmio_addr: VirtualAddress = VirtualAddress::new(raw_vaddr);
                    let phys_addr: PhysicalAddress =
                        // FIXME: ensure safety here.
                        unsafe { PhysicalAddress::from_mmio_address(mmio_addr) };
                    FrameAddress::new(PageAligned::from_address(phys_addr)?)
                };
            }
        }
    }

    Ok(root_pagetables)
}
