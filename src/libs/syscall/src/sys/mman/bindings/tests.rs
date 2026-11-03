// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
