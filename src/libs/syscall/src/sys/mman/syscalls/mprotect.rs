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
use ::config::memory_layout::USER_MMAP_END_RAW;
use ::spin::MutexGuard;
use ::sys::{
    config::memory_layout::{
        USER_MMAP_BASE,
        USER_MMAP_END,
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
    if base < USER_MMAP_BASE || base >= USER_MMAP_END {
        // POSIX specifies that calling `mprotect()` on an address range that was not previously
        // mapped with `mmap()` has undefined behavior. However, Linux does allow such an operation
        // to succeed. To be compatible with Linux, we simply don't change any protection flags.
        // See: https://www.man7.org/linux/man-pages/man2/mprotect.2.html
        return Ok(());
    }

    // Check if base address is page-aligned.
    if !base.is_aligned(PAGE_ALIGNMENT) {
        let reason: &'static str = "base address is not page-aligned";
        ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Check if end address is invalid.
    let end: VirtualAddress = match base.into_raw_value().checked_add(len) {
        Some(end) if end > USER_MMAP_END_RAW => {
            let reason: &'static str = "invalid end address";
            ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        },
        None => {
            let reason: &'static str = "address overflow";
            ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        },
        Some(end) => VirtualAddress::from_raw_value(end),
    };

    // Check if length is page-aligned.
    if !len.is_multiple_of(usize::from(PAGE_ALIGNMENT)) {
        let reason: &'static str = "length is not page-aligned";
        ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Lock the segments map.
    let mut segments: MutexGuard<'_, BTreeMap<VirtualAddress, MemorySegment>> =
        MMAP_SEGMENTS.lock();

    // Find the segment that contains the base address.
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
            let segment: &mut MemorySegment = if let Some(segment) = segments.get_mut(&segment_base)
            {
                segment
            } else {
                let reason: &'static str = "memory segment not found";
                ::syslog::error!("mprotect(): {reason} (base={base:?}, len={len}, prot={prot:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            };

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
