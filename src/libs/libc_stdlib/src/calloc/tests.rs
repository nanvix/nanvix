// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::calloc;
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
fn null_allocation() {
    set_errno(0);
    let p = unsafe { calloc(0, 64) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn zero_size_allocation() {
    set_errno(0);
    let p = unsafe { calloc(32, 0) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn valid_allocation() {
    let nmemb: c_size_t = 16;
    let size: c_size_t = 8;
    let total: usize = (nmemb * size) as usize;
    let p = unsafe { calloc(nmemb, size) }.cast::<u8>();
    assert!(!p.is_null());
    unsafe {
        for i in 0..total {
            assert_eq!(p.add(i).read(), 0u8, "calloc memory not zero at {i}");
        }
    }
    unsafe {
        crate::free(p.cast());
    }
}
