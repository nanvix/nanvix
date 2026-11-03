// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::aligned_alloc;
use crate::set_errno;
use ::sysapi::{
    errno::EINVAL,
    ffi::c_int,
    sys_types::c_size_t,
};

// Helper to read errno safely.
fn get_errno() -> c_int {
    unsafe { *sysapi::errno::__errno_location() }
}

#[test]
fn zero_size_allocation() {
    set_errno(0);
    let p = unsafe { aligned_alloc(64, 0) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn null_alignment() {
    set_errno(0);
    let p = unsafe { aligned_alloc(0, 64) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn valid_allocation_multiple() {
    let alignment: c_size_t = 64;
    let size: c_size_t = 256; // multiple of alignment
    let p = unsafe { aligned_alloc(alignment, size) }.cast::<u8>();
    assert!(!p.is_null());
    let addr = p as usize;
    assert_eq!(addr & (alignment as usize - 1), 0, "pointer {addr:#x} not {alignment}-aligned");
    unsafe {
        crate::free(p.cast());
    }
}

#[test]
fn size_not_multiple_of_alignment_fails() {
    set_errno(0);
    let alignment: c_size_t = 64;
    let size: c_size_t = 130; // not a multiple of alignment
    let p = unsafe { aligned_alloc(alignment, size) }.cast::<u8>();
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}
