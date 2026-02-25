// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_ALIGNMENT;
use ::spin::{
    Lazy,
    Mutex,
    MutexGuard,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        self,
        Address,
        VirtualAddress,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Base virtual address for the unified mmap region.
const MMAP_BASE: VirtualAddress = VirtualAddress::new(::config::memory_layout::USER_MMAP_BASE_RAW);

/// End of the unified mmap region.
const MMAP_END_RAW: usize = ::config::memory_layout::USER_MMAP_END_RAW;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Bump allocator for the unified virtual address region.
///
/// TODO: This is a bump-only allocator that never reclaims virtual address space.
/// Long-running processes that load/unload many shared libraries or perform many mmap/munmap
/// cycles will eventually exhaust the region even though physical pages were returned.
/// Replace with a proper free-list or bitmap allocator to support VA reuse.
static VADDR_NEXT: Lazy<Mutex<VirtualAddress>> = Lazy::new(|| Mutex::new(MMAP_BASE));

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reserves a contiguous block of virtual address space from the unified mmap region without
/// mapping any physical pages. The caller is responsible for lazily mapping pages within the
/// returned range (e.g., via kernel calls).
///
/// # Parameters
///
/// - `length`: Size of the virtual address range to reserve in bytes (will be page-aligned upward).
///
/// # Returns
///
/// On success, this function returns the base address of the reserved virtual address range.
/// On failure, it returns an [`Error`] indicating the reason for the failure.
///
pub fn reserve(length: usize) -> Result<VirtualAddress, Error> {
    // Reject zero-length reservations.
    if length == 0 {
        let reason: &str = "length must be greater than zero";
        ::syslog::error!("vaddr::reserve(): {reason} (length={length})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Align up length to page size.
    let length: usize = mm::align_up(length, PAGE_ALIGNMENT).ok_or_else(|| {
        let reason: &str = "align_up overflow";
        ::syslog::error!("vaddr::reserve(): {reason} (length={length})");
        Error::new(ErrorCode::InvalidArgument, reason)
    })?;

    let mut locked_next: MutexGuard<'_, VirtualAddress> = VADDR_NEXT.lock();

    // Compute the new base address.
    let base_raw: usize = locked_next.into_raw_value();
    let new_base_raw: usize = base_raw.checked_add(length).ok_or_else(|| {
        let reason: &str = "address overflow when reserving virtual address space";
        ::syslog::error!("vaddr::reserve(): {reason} (length={length})");
        Error::new(ErrorCode::OutOfMemory, reason)
    })?;

    // Check if we have enough space.
    if new_base_raw > MMAP_END_RAW {
        let reason: &str = "not enough virtual address space for reservation";
        ::syslog::error!("vaddr::reserve(): {reason} (length={length})");
        return Err(Error::new(ErrorCode::OutOfMemory, reason));
    }

    let reserved_base: VirtualAddress = *locked_next;

    // Bump the allocator.
    *locked_next = VirtualAddress::new(new_base_raw);

    Ok(reserved_base)
}
