// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

// Bindings that issue kernel calls are only available with the `syscall` feature.
#[cfg(feature = "syscall")]
pub mod mmap;
#[cfg(feature = "syscall")]
pub mod mprotect;
#[cfg(feature = "syscall")]
pub mod munmap;

// `mlock()`, `munlock()`, and `msync()` are no-ops after argument validation. They share the
// host-testable helpers below, so they are compiled under the relaxed module gate (see the parent
// module) rather than being restricted to the `syscall` feature.
pub mod mlock;
pub mod msync;
pub mod munlock;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(any(feature = "syscall", test))]
use ::sys::error::ErrorCode;
#[cfg(any(feature = "syscall", test))]
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Validates and normalizes the address range supplied to the `mlock()` and `munlock()` bindings.
///
/// Nanvix never swaps or pages out memory, so every mapped page is permanently resident. Both
/// bindings therefore share this routine, whose purpose is to reject malformed ranges and round the
/// non-empty range up to the whole pages that POSIX requires the operation to cover:
///
/// - A zero-length range covers no pages and is always accepted.
/// - POSIX permits requiring the base address to be a multiple of the page size, so a misaligned
///   `addr` is rejected with `ErrorCode::InvalidArgument` (`EINVAL`).
/// - A range that extends past the end of the address space cannot correspond to valid mapped
///   pages and is rejected with `ErrorCode::OutOfMemory` (`ENOMEM`).
///
/// # Parameters
///
/// - `addr`: Base address of the range, as an integer.
/// - `length`: Number of bytes in the range.
///
/// # Returns
///
/// Returns `Ok(None)` for a zero-length range, `Ok(Some((base, length)))` for a normalized
/// non-empty range, otherwise the `ErrorCode` to report via `errno`.
///
#[cfg(any(feature = "syscall", test))]
fn validate_lock_range(addr: usize, length: c_size_t) -> Result<Option<(usize, usize)>, ErrorCode> {
    // A zero-length range covers no pages; there is nothing to lock or unlock.
    if length == 0 {
        return Ok(None);
    }

    // Convert the length to the native word size for range arithmetic.
    let length: usize = match usize::try_from(length) {
        Ok(length) => length,
        // A length that does not fit in the address space cannot correspond to valid pages.
        Err(_) => return Err(ErrorCode::OutOfMemory),
    };

    // POSIX permits requiring the base address to be a multiple of the page size.
    if !addr.is_multiple_of(::arch::mem::PAGE_SIZE) {
        return Err(ErrorCode::InvalidArgument);
    }

    // POSIX operates on whole pages containing any byte in the requested range.
    let length: usize = match ::sys::mm::align_up(length, ::arch::mem::PAGE_ALIGNMENT) {
        Some(length) => length,
        None => return Err(ErrorCode::OutOfMemory),
    };

    // A range that wraps past the end of the address space cannot correspond to valid pages.
    if addr.checked_add(length).is_none() {
        return Err(ErrorCode::OutOfMemory);
    }

    Ok(Some((addr, length)))
}

///
/// # Description
///
/// Checks whether a normalized lock range is fully covered by mapped memory.
///
/// Coverage may be provided by a single mapped segment or by several adjacent segments whose
/// combined extents leave no gap within the range.
///
/// # Parameters
///
/// - `addr`: Page-aligned base address of the range.
/// - `length`: Page-rounded length of the range.
/// - `segments`: Iterator over mapped `(base, length)` pairs in ascending base-address order.
///
/// # Returns
///
/// Returns `true` if the range is fully covered by one or more adjacent mapped segments, otherwise
/// `false`.
///
#[cfg(any(feature = "syscall", test))]
pub(super) fn is_lock_range_mapped<I>(addr: usize, length: usize, segments: I) -> bool
where
    I: IntoIterator<Item = (usize, usize)>,
{
    let end: usize = match addr.checked_add(length) {
        Some(end) => end,
        None => return false,
    };

    let mut cursor: usize = addr;

    for (segment_base, segment_length) in segments {
        let segment_end: usize = match segment_base.checked_add(segment_length) {
            Some(segment_end) => segment_end,
            None => return false,
        };

        if segment_end <= cursor {
            continue;
        }

        if segment_base > cursor {
            return false;
        }

        if segment_end >= end {
            return true;
        }

        cursor = segment_end;
    }

    false
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        is_lock_range_mapped,
        validate_lock_range,
    };
    use ::arch::mem::PAGE_SIZE;
    use ::sys::error::ErrorCode;
    use ::sysapi::sys_types::c_size_t;

    /// A page-aligned, in-bounds range is accepted; locking is a successful no-op.
    #[test]
    fn accepts_page_aligned_range() {
        assert_eq!(validate_lock_range(PAGE_SIZE, 1), Ok(Some((PAGE_SIZE, PAGE_SIZE))));
    }

    /// A zero-length range covers no pages and is accepted regardless of alignment.
    #[test]
    fn accepts_zero_length_range() {
        assert_eq!(validate_lock_range(PAGE_SIZE + 1, 0), Ok(None));
    }

    /// A range that touches a second page is rounded up to cover both pages.
    #[test]
    fn rounds_range_up_to_whole_pages() {
        let length: c_size_t = c_size_t::try_from(PAGE_SIZE + 1).unwrap_or(c_size_t::MAX);

        assert_eq!(validate_lock_range(PAGE_SIZE, length), Ok(Some((PAGE_SIZE, 2 * PAGE_SIZE))));
    }

    /// A base address that is not a multiple of the page size is rejected with `EINVAL`.
    #[test]
    fn rejects_unaligned_address() {
        assert_eq!(validate_lock_range(PAGE_SIZE + 1, 1), Err(ErrorCode::InvalidArgument));
    }

    /// A range that extends past the end of the address space is rejected with `ENOMEM`.
    #[test]
    fn rejects_overflowing_range() {
        // Largest page-aligned base address; locking one more page runs past the address space.
        let base: usize = usize::MAX & !(PAGE_SIZE - 1);
        let length: c_size_t = c_size_t::try_from(PAGE_SIZE).unwrap_or(c_size_t::MAX);
        assert_eq!(validate_lock_range(base, length), Err(ErrorCode::OutOfMemory));
    }

    /// A normalized range fully covered by one mapped segment is accepted.
    #[test]
    fn accepts_mapped_range() {
        assert!(is_lock_range_mapped(PAGE_SIZE, PAGE_SIZE, [(PAGE_SIZE, PAGE_SIZE)]));
    }

    /// A normalized range fully covered by adjacent mapped segments is accepted.
    #[test]
    fn accepts_range_spanning_adjacent_mapped_segments() {
        assert!(is_lock_range_mapped(
            PAGE_SIZE,
            2 * PAGE_SIZE,
            [(PAGE_SIZE, PAGE_SIZE), (2 * PAGE_SIZE, PAGE_SIZE)]
        ));
    }

    /// A range with no covering segment is rejected with `ENOMEM` by the caller.
    #[test]
    fn rejects_unmapped_range() {
        assert!(!is_lock_range_mapped(PAGE_SIZE, PAGE_SIZE, core::iter::empty()));
    }

    /// A range that extends past the mapped segment is rejected with `ENOMEM` by the caller.
    #[test]
    fn rejects_partially_mapped_range() {
        assert!(!is_lock_range_mapped(PAGE_SIZE, 2 * PAGE_SIZE, [(PAGE_SIZE, PAGE_SIZE)]));
    }
}
