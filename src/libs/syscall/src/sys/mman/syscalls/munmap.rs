// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::mem::segment::MemorySegment,
    sys::mman::syscalls::MMAP_SEGMENTS,
};
use ::alloc::collections::BTreeMap;
use ::arch::mem::PAGE_ALIGNMENT;
use ::spin::MutexGuard;
use ::sys::{
    error::Error,
    mm,
    mm::{
        Address,
        VirtualAddress,
    },
};
use config::memory_layout::USER_MMAPPED_END_RAW;
use sys::{
    config::memory_layout::USER_MMAP_BASE,
    error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Unmaps a memory segment.
///
/// # Parameters
///
/// - `base`: Base address of the memory segment to be unmapped.
/// - `length`: Length of the memory segment to be unmapped.
///
/// # Returns
///
/// On success, this function returns empty. On failure, it returns an `Error` indicating the reason
/// for the failure.
///
/// # Known Limitations
///
/// - Partial unmapping is not supported. If the requested length is less than the segment's capacity,
///   the entire segment will be unmapped.
///
pub fn munmap(base: VirtualAddress, length: usize) -> Result<(), Error> {
    ::syslog::trace!("munmap(): base={base:?}, length={length}");

    // Align up length to page size.
    let length: usize = mm::align_up(length, PAGE_ALIGNMENT);

    // Check if the base address is valid.
    match base.into_raw_value().checked_add(length) {
        Some(end_addr) if base >= USER_MMAP_BASE && end_addr <= USER_MMAPPED_END_RAW => {},
        _ => {
            let reason: &str = "base address out of bounds";
            syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
    }

    // Lock the segments map.
    let mut segments: MutexGuard<'_, BTreeMap<VirtualAddress, MemorySegment>> =
        MMAP_SEGMENTS.lock();

    // Check if the segment exists.
    if let Some(segment) = segments.get(&base) {
        // Check if the segment is large enough.
        if segment.capacity() < length {
            let reason: &str = "segment is too small";
            syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Remove the segment from the map.
        segments.remove(&base);

        // Segment is unmapped when this scope ends.

        Ok(())
    } else {
        let reason: &str = "segment not found";
        syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
        Err(Error::new(ErrorCode::InvalidArgument, reason))
    }
}
