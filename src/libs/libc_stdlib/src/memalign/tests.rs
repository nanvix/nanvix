// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::memalign;
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
    let p = unsafe { memalign(64, 0) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn null_alignment() {
    set_errno(0);
    let p = unsafe { memalign(0, 64) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn non_power_of_two_alignment() {
    set_errno(0);
    let p = unsafe { memalign(24, 128) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn alignment_above_maximum() {
    // A power-of-two alignment beyond the allocator's maximum must fail with EINVAL rather
    // than the ENOMEM the allocator would otherwise surface.
    set_errno(0);
    let p = unsafe { memalign(8192, 128) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn valid_allocation() {
    let alignment: c_size_t = 128;
    let size: c_size_t = 130; // not a multiple of alignment
    let p = unsafe { memalign(alignment, size) }.cast::<u8>();
    assert!(!p.is_null());
    let addr = p as usize;
    assert_eq!(addr & (alignment as usize - 1), 0, "pointer {addr:#x} not {alignment}-aligned");
    unsafe {
        crate::free(p.cast());
    }
}
