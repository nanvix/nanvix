// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::mem::segment::MemorySegment,
    sys::mman::{
        syscalls::{
            mmap_reserve,
            MMAP_SEGMENTS,
        },
        MemoryMapProtectionFlags,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::VirtualAddress,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Maps a new memory segment.
///
/// # Parameters
///
/// - `length`: Length of the memory segment to be mapped.
/// - `prot`: Protection flags for the memory segment.
///
/// # Returns
///
/// On success, this function returns the base address of the newly mapped memory segment.
/// On failure, it returns an `Error` indicating the reason for the failure.
///
pub fn mmap(length: usize, prot: MemoryMapProtectionFlags) -> Result<VirtualAddress, Error> {
    ::syslog::trace!("mmap(): length={length}, prot={prot:?}");

    // Page-align the requested length upfront so the same value is used for both the virtual
    // address reservation and the physical page mapping.
    let aligned_length: usize = ::sys::mm::align_up(length, ::arch::mem::PAGE_ALIGNMENT)
        .ok_or_else(|| {
            let reason: &str = "align_up overflow";
            ::syslog::error!("mmap(): {reason} (length={length})");
            Error::new(ErrorCode::InvalidArgument, reason)
        })?;

    // Reserve virtual address space from the unified mmap region.
    let segment_base: VirtualAddress = mmap_reserve(aligned_length)?;

    // Attempt to allocate a new memory segment (maps physical pages).
    let segment: MemorySegment = MemorySegment::new(segment_base, aligned_length, prot.into())?;

    // Add new segment to the map of memory segments.
    MMAP_SEGMENTS.lock().insert(segment.base(), segment);

    Ok(segment_base)
}
