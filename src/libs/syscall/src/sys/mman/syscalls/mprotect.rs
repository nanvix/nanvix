// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::mem::segment::MemorySegment,
    sys::mman::{
        syscalls::MMAP_SEGMENTS,
        MemoryMapProtectionFlags,
    },
};
use ::alloc::collections::BTreeMap;
use ::arch::mem::PAGE_ALIGNMENT;
use ::config::memory_layout::USER_MMAPPED_END_RAW;
use ::spin::MutexGuard;
use ::sys::{
    config::memory_layout::{
        USER_MMAPPED_END,
        USER_MMAP_BASE,
    },
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        Address,
        VirtualAddress,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the protection of a memory segment.
///
/// # Parameters
///
/// - `base`: Base address of the memory segment.
/// - `len`: Length of the memory segment.
/// - `prot`: New protection flags.
///
/// # Returns
///
/// On success, this function returns empty. On failure, it returns an `Error` indicating the reason
/// for the failure.
///
/// # Known Limitations
///
/// - Partial protection changes are not supported. If the requested length is less than the
///   segment's capacity, the entire segment's protection will be changed.
///
pub fn mprotect(
    base: VirtualAddress,
    len: usize,
    prot: MemoryMapProtectionFlags,
) -> Result<(), Error> {
    // Check if base address is invalid.
    if base < USER_MMAP_BASE || base >= USER_MMAPPED_END {
        let reason: &'static str = "invalid base address";
        ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Check if base address is page-aligned.
    if !base.is_aligned(PAGE_ALIGNMENT) {
        let reason: &'static str = "base address is not page-aligned";
        ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Check if end address is invalid.
    match base.into_raw_value().checked_add(len) {
        Some(end) if end > USER_MMAPPED_END_RAW => {
            let reason: &'static str = "invalid end address";
            ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        },
        None => {
            let reason: &'static str = "address overflow";
            ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        },
        Some(_valid_base_addr) => (),
    }

    // Check if length is page-aligned.
    if len % usize::from(PAGE_ALIGNMENT) != 0 {
        let reason: &'static str = "length is not page-aligned";
        ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Lock the segments map.
    let mut segments: MutexGuard<'_, BTreeMap<VirtualAddress, MemorySegment>> =
        MMAP_SEGMENTS.lock();

    // Find the segment that contains the base address.
    let segment_base: Option<VirtualAddress> = segments
        .iter()
        .find(|(_, segment)| {
            let seg_base: VirtualAddress = segment.base();
            let seg_end: VirtualAddress = seg_base + segment.capacity();
            base >= seg_base && (base + len) <= seg_end
        })
        .map(|(seg_base, _)| *seg_base);

    match segment_base {
        Some(segment_base) => {
            let segment: &mut MemorySegment = if let Some(segment) = segments.get_mut(&segment_base)
            {
                segment
            } else {
                let reason: &'static str = "memory segment not found";
                ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            };

            // Check if the segment is large enough.
            if segment.capacity() < len || base < segment.base() {
                let reason: &'static str = "segment is too small or base not aligned with segment";
                ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }

            // Check for partial mapping.
            if segment.capacity() != len || segment.base() != base {
                ::syslog::warn!(
                    "mprotect(): partial mapping of segment (base={base:?}, len={len}, \
                     prot={prot:?})"
                );

                // TODO (#992): split the segment and change protection only for the requested part.
            }

            segment.set_protection(prot.into())
        },
        None => {
            let reason: &'static str = "memory segment not found";
            ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
            Err(Error::new(ErrorCode::OutOfMemory, reason))
        },
    }
}
