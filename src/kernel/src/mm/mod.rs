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
mod kernel_vas;
pub(crate) mod phys;
mod virt;

//==================================================================================================
// Exports
//==================================================================================================

pub mod kheap;
use ::arch::mem::{
    PAGE_ALIGNMENT,
    PGTAB_ALIGNMENT,
};
pub(crate) use virt::{
    GetPageDirectoryStorage,
    GetPageTableStorage,
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

use crate::hal::mem::{
    PhysicalAddress,
    TruncatedMemoryRegion,
    VirtualAddress,
};
use ::alloc::{
    boxed::Box,
    collections::LinkedList,
    string::String,
    vec::Vec,
};
use ::arch::mem;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Re-exports
//==================================================================================================

pub use kernel_vas::init;

//==================================================================================================
// Static Assertions
//==================================================================================================

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
// Type Aliases
//==================================================================================================

type VirtMemRegion = LinkedList<TruncatedMemoryRegion<VirtualAddress>>;
type PhysMemRegion = LinkedList<TruncatedMemoryRegion<PhysicalAddress>>;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Builds an [`ErrorCode::OutOfMemory`] error for a failed kernel heap allocation.
fn out_of_memory() -> Error {
    Error::new(ErrorCode::OutOfMemory, "failed to allocate memory on kernel heap")
}

//--------------------------------------------------------------------------------------------------
// Box
//--------------------------------------------------------------------------------------------------

/// Allocates `value` in a [`Box`] on the kernel heap using fallible allocation.
///
/// Unlike [`Box::new`], this returns an [`ErrorCode::OutOfMemory`] error when the allocation fails
/// instead of aborting, so callers can propagate the failure.
pub fn try_box<T>(value: T) -> Result<Box<T>, Error> {
    Box::try_new(value).map_err(|_| out_of_memory())
}

//--------------------------------------------------------------------------------------------------
// Vec
//--------------------------------------------------------------------------------------------------

/// Creates an empty [`Vec`] with at least `capacity` elements pre-allocated on the kernel heap
/// using fallible allocation.
///
/// Unlike [`Vec::with_capacity`], this returns an [`ErrorCode::OutOfMemory`] error when the
/// allocation fails instead of aborting, so callers can propagate the failure.
pub fn try_vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, Error> {
    let mut vec: Vec<T> = Vec::new();
    vec.try_reserve_exact(capacity)
        .map_err(|_| out_of_memory())?;
    Ok(vec)
}

//--------------------------------------------------------------------------------------------------
// String
//--------------------------------------------------------------------------------------------------

/// Creates an empty [`String`] with at least `capacity` bytes pre-allocated on the kernel heap
/// using fallible allocation.
///
/// Unlike [`String::with_capacity`], this returns an [`ErrorCode::OutOfMemory`] error when the
/// allocation fails instead of aborting, so callers can propagate the failure.
pub fn try_string_with_capacity(capacity: usize) -> Result<String, Error> {
    let mut string: String = String::new();
    string
        .try_reserve_exact(capacity)
        .map_err(|_| out_of_memory())?;
    Ok(string)
}

/// Creates a [`String`] containing a copy of `value` on the kernel heap using fallible allocation.
///
/// Unlike [`str::to_string`], this returns an [`ErrorCode::OutOfMemory`] error when the allocation
/// fails instead of aborting, so callers can propagate the failure.
pub fn try_string_from_str(value: &str) -> Result<String, Error> {
    let mut string: String = try_string_with_capacity(value.len())?;
    string.push_str(value);
    Ok(string)
}
