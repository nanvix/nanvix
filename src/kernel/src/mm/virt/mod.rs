// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub(crate) mod identity_map;
mod kpage;
mod manager;
pub(crate) mod page_table_allocator;
mod vmem;

pub(crate) use identity_map::{
    identity_map_page,
    memcpy,
    memset,
};

//==================================================================================================
// Imports
//==================================================================================================

use self::page_table_allocator::alloc_page_table_slot;
use crate::hal::{
    arch::x86::mem::mmu::page_table::PageTable,
    mem::{
        Address,
        FrameAddress,
        PageTableAddress,
        PageTableAligned,
        TruncatedMemoryRegion,
        VirtualAddress,
    },
};
use ::alloc::collections::LinkedList;
use ::arch::mem::{
    self,
    paging::PteWord,
    PAGE_TABLE_LENGTH,
};
use ::core::ops::{
    Deref,
    DerefMut,
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
// Structures and Enums
//==================================================================================================

pub enum PageTableStorage {
    /// Boot-time BSS-backed storage, allocated via `PAGE_TABLE_ALLOCATOR`.
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

// FIXME: this function is too long and complex.
pub fn init(
    virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<LinkedList<(PageTableAligned<VirtualAddress>, PageTableStorage)>, Error> {
    info!("initializing kernel page tables ...");

    let mut root_pagetables: LinkedList<(PageTableAligned<VirtualAddress>, PageTableStorage)> =
        LinkedList::new();

    // Sort regions by start address so we can detect when adjacent regions share a PT.
    let mut regions: alloc::vec::Vec<_> = virtual_memory_regions.into_iter().collect();
    regions.sort();

    // Allocate PTs only for the provided kernel regions. The lazy identity mapper
    // (`identity_map::ensure_identity_mapped`) will allocate PTs from the kernel page
    // pool on demand for any physical address outside these regions.
    for region in regions.iter() {
        info!("booking: {:?}", region);
        let start: usize = region.start().into_raw_value();
        let end: usize = start + region.size();
        let mut cur: usize = start;

        while cur < end && cur < config::kernel::MEMORY_SIZE {
            let pt_base: usize = (cur / mem::PGTAB_SIZE) * mem::PGTAB_SIZE;

            // Allocate a new PT if the current address isn't covered by the last one.
            let need_new_pt: bool = match root_pagetables.back() {
                Some((addr, _)) => addr.into_raw_value() != pt_base,
                None => true,
            };

            if need_new_pt {
                let storage: PageTableStorage =
                    // SAFETY: called during single-threaded kernel init;
                    // BSS is zero-initialized, so assume_init_mut() is sound.
                    PageTableStorage::Bss(unsafe { alloc_page_table_slot() });
                // SAFETY: storage address is valid and identity-mapped (still on boot page tables).
                let mut pt: PageTable = unsafe {
                    PageTable::from_address(PageTableAddress::from_raw_value(
                        storage.as_ptr() as usize
                    )?)
                };
                pt.clean();
                let pt_addr: PageTableAligned<VirtualAddress> =
                    PageTableAligned::from_address(VirtualAddress::new(pt_base))?;
                root_pagetables.push_back((pt_addr, storage));
            }

            // Fill PTEs in the current PT for this region.
            let (_, storage) = root_pagetables
                .back()
                .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "no page table available"))?;
            let mut pt: PageTable = unsafe {
                PageTable::from_address(
                    PageTableAddress::from_raw_value(storage.as_ptr() as usize)?,
                )
            };
            let pte_start: usize = (cur - pt_base) / mem::PAGE_SIZE;
            let pte_end: usize = PAGE_TABLE_LENGTH.min(
                pte_start + (end.min(config::kernel::MEMORY_SIZE) - cur).div_ceil(mem::PAGE_SIZE),
            );
            let count: usize = pte_end - pte_start;

            if count > 0 {
                pt.fill(pte_start, count, FrameAddress::from_raw_value(cur)?, true, false)
                    .map_err(|(_, e)| e)?;
            }

            cur += count * mem::PAGE_SIZE;
        }
    }

    Ok(root_pagetables)
}

//==================================================================================================
// Tests
//==================================================================================================

/// Runs the virtual-memory subsystem unit tests.
#[cfg(feature = "test")]
pub fn test() {
    assert!(identity_map::test::test());
}
