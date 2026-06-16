// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Description
//!
//! This module instantiates a global fixed-size bump allocator for kernel page tables. Allocation
//! is backed by statically reserved and aligned BSS storage and is used both during boot and
//! after boot for lazy identity-map page table allocation.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::{
    self,
    paging::{
        self,
        PteWord,
    },
    PAGE_TABLE_LENGTH,
};
use ::bump_allocator::{
    align_up,
    BssStorage,
    FixedSizeBumpAllocator,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of page tables for identity-mapping all physical memory.
const NUM_PAGE_TABLES: usize = crate::hal::platform::NUM_PAGE_TABLES;

/// Total number of kernel page-table-sized slots.
///
/// This covers the page tables for identity-mapping all physical memory, plus structures for the
/// root paging hierarchy.
const NUM_KERNEL_PAGE_TABLES: usize = NUM_PAGE_TABLES + paging::NUM_HIERARCHY_PAGES;

/// Slot size used by the page-table allocator (one page-table-sized unit).
const PAGE_TABLE_SLOT_SIZE: usize = core::mem::size_of::<[PteWord; PAGE_TABLE_LENGTH]>();

/// Slot alignment used by the page-table allocator.
const PAGE_TABLE_SLOT_ALIGN: usize = mem::PAGE_SIZE;

/// Distance between consecutive page-table slots.
const PAGE_TABLE_SLOT_STRIDE: usize = match align_up(PAGE_TABLE_SLOT_SIZE, PAGE_TABLE_SLOT_ALIGN) {
    Some(v) => v,
    None => panic!("page table slot stride overflow"),
};

/// Total size of BSS storage reserved for all page-table slots.
const PAGE_TABLE_STORAGE_SIZE: usize = NUM_KERNEL_PAGE_TABLES * PAGE_TABLE_SLOT_STRIDE;

//==================================================================================================
// Static Assertions
//==================================================================================================

// Ensure that the kernel memory size is a multiple of a page table size.
::static_assert::assert_eq!(config::kernel::MEMORY_SIZE.is_multiple_of(mem::PGTAB_SIZE));

//==================================================================================================
// Page Table BSS Storage
//==================================================================================================

/// Page-aligned BSS storage for all page table allocator slots.
#[repr(align(4096))]
struct PageTableBssStorage {
    bytes: [u8; PAGE_TABLE_STORAGE_SIZE],
}

::static_assert::assert_eq_align!(PageTableBssStorage, mem::PAGE_SIZE);

/// SAFETY: storage is accessed exclusively through the allocator's atomic bump index.
static mut PAGE_TABLE_STORAGE: PageTableBssStorage = PageTableBssStorage {
    bytes: [0; PAGE_TABLE_STORAGE_SIZE],
};

/// Storage provider for page-table allocator slots.
pub struct PageTableBss;

// SAFETY: PAGE_TABLE_STORAGE is a stable static backing region exclusively managed by
// PAGE_TABLE_ALLOCATOR.
unsafe impl BssStorage for PageTableBss {
    const NUM_UNITS: usize = NUM_KERNEL_PAGE_TABLES;
    const STORAGE_SIZE: usize = PAGE_TABLE_STORAGE_SIZE;

    fn as_mut_ptr() -> *mut u8 {
        // SAFETY: raw pointer creation does not create aliases by itself.
        unsafe { core::ptr::addr_of_mut!(PAGE_TABLE_STORAGE.bytes) as *mut u8 }
    }
}

/// Global fixed-size allocator used for kernel page-table slots during boot and after boot for
/// lazy identity-map page table allocation.
/// SAFETY: PAGE_TABLE_ALLOCATOR is a global allocator that exclusively manages PAGE_TABLE_STORAGE.
pub static PAGE_TABLE_ALLOCATOR: FixedSizeBumpAllocator<
    PAGE_TABLE_SLOT_SIZE,
    PAGE_TABLE_SLOT_ALIGN,
    PageTableBss,
> = unsafe { FixedSizeBumpAllocator::new() };

///
/// # Description
///
/// Allocates a zeroed, page-aligned page-table slot from BSS storage.
///
/// This is a convenience wrapper around [`PAGE_TABLE_ALLOCATOR`] that returns a direct
/// mutable reference to a `[PteWord; PAGE_TABLE_LENGTH]` array.
///
/// # Safety
///
/// The caller must ensure the returned slot is used exclusively (no aliasing).
///
/// # Panics
///
/// Panics if all page-table slots have been exhausted.
///
pub unsafe fn alloc_page_table_slot() -> &'static mut [PteWord; PAGE_TABLE_LENGTH] {
    PAGE_TABLE_ALLOCATOR
        .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
        .expect("BSS page table allocation failed")
        .assume_init_mut()
}
