// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::mem::segment::MemorySegment,
    sys::mman::{
        syscalls::{
            MMAP_BASE,
            MMAP_SEGMENTS,
        },
        MemoryMapProtectionFlags,
    },
};
use ::arch::mem::PAGE_ALIGNMENT;
use ::config::memory_layout::USER_MMAPPED_END_RAW;
use ::spin::MutexGuard;
use ::sys::{
    error::Error,
    mm,
    mm::{
        Address,
        VirtualAddress,
    },
};
use sys::error::ErrorCode;

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

    // Reject zero-length mappings.
    if length == 0 {
        let reason: &str = "length must be greater than zero";
        syslog::error!("mmap(): {reason} (length={length}, prot={prot:?})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Align up length to page size.
    let length: usize = mm::align_up(length, PAGE_ALIGNMENT);

    // Lock the segments map.
    let mut locked_mmap_base: MutexGuard<'_, VirtualAddress> = MMAP_BASE.lock();

    // Compute new mmap base address checking for overflow.
    let new_mmap_base: VirtualAddress = {
        let base_raw: usize = locked_mmap_base.into_raw_value();
        let new_mmap_base_raw: Option<usize> = base_raw.checked_add(length);
        match new_mmap_base_raw {
            Some(addr) => {
                // Check if we have enough space for the new memory segment.
                if addr >= USER_MMAPPED_END_RAW {
                    let reason: &str = "not enough space for new memory segment";
                    syslog::error!("mmap(): {reason} (length={length}, prot={prot:?})");
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                }
                VirtualAddress::new(addr)
            },
            None => {
                let reason: &str = "address overflow when mapping new memory segment";
                syslog::error!("mmap(): {reason} (length={length}, prot={prot:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        }
    };
    let segment_base: VirtualAddress = *locked_mmap_base;

    // Attempt to allocate a new memory segment.
    let segment: MemorySegment = MemorySegment::new(segment_base, length, prot.into())?;

    // Add new segment to the map of memory segments.
    MMAP_SEGMENTS.lock().insert(segment.base(), segment);

    // Bump the base address for the next allocation.
    *locked_mmap_base = new_mmap_base;

    Ok(segment_base)
}
