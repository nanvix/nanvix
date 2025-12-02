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
    mm::{
        Address,
        VirtualAddress,
    },
};
use config::memory_layout::USER_MMAP_END_RAW;
use sys::{
    config::memory_layout::{
        USER_MMAP_BASE,
        USER_MMAP_END,
    },
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

    // Check if the base address is valid.
    if base < USER_MMAP_BASE || base >= USER_MMAP_END {
        let reason: &str = "invalid base address";
        syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Check if base address is page-aligned.
    if !base.is_aligned(PAGE_ALIGNMENT) {
        let reason: &str = "base address is not page-aligned";
        syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Check if end address is invalid.
    let end: VirtualAddress = match base.into_raw_value().checked_add(length) {
        Some(end) if end > USER_MMAP_END_RAW => {
            let reason: &str = "invalid end address";
            syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
        None => {
            let reason: &str = "end address overflow";
            syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
        Some(base_end) => VirtualAddress::from_raw_value(base_end),
    };

    // Check if length is page-aligned.
    if !length.is_multiple_of(usize::from(PAGE_ALIGNMENT)) {
        let reason: &str = "length is not page-aligned";
        syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Lock the segments map.
    let mut segments: MutexGuard<'_, BTreeMap<VirtualAddress, MemorySegment>> =
        MMAP_SEGMENTS.lock();

    // Find the segment that contains the base address.
    // Use BTreeMap's range query to efficiently find the segment containing the base address.
    let segment_base: Option<VirtualAddress> =
        segments
            .range(..=base)
            .next_back()
            .and_then(|(seg_base, segment)| {
                let seg_end: Option<VirtualAddress> = seg_base
                    .into_raw_value()
                    .checked_add(segment.capacity())
                    .map(VirtualAddress::from_raw_value);
                match seg_end {
                    Some(seg_end_addr) => {
                        if base >= *seg_base && end <= seg_end_addr {
                            Some(*seg_base)
                        } else {
                            None
                        }
                    },
                    // Overflow occurred, treat as not found.
                    None => None,
                }
            });

    match segment_base {
        Some(segment_base) => {
            let segment: &MemorySegment = &segments[&segment_base];

            // Check for partial mapping.
            if segment.capacity() != length || segment.base() != base {
                syslog::warn!(
                    "munmap(): partial unmapping is not supported (base={base:?}, \
                     length={length}, segment_base={segment_base:?}, segment_capacity={})",
                    segment.capacity()
                );

                // TODO (#992): split the segment and fall through instead of returning.
                return Ok(());
            }

            // Remove the segment from the map.
            debug_assert!(segments.remove(&segment_base).is_some());

            // Segment is unmapped when this scope ends.

            Ok(())
        },
        None => {
            let reason: &str = "segment not found";
            syslog::error!("munmap(): {reason} (base={base:?}, length={length})");
            Err(Error::new(ErrorCode::InvalidArgument, reason))
        },
    }
}
