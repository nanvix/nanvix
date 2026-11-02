// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::malloc;
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
fn null_storage() {
    set_errno(0);
    let p = unsafe { malloc(0) };
    assert!(p.is_null());
    assert_eq!(get_errno(), EINVAL);
}

#[test]
fn valid_allocation() {
    let size: c_size_t = 128;
    let p = unsafe { malloc(size) };
    assert!(!p.is_null());
    assert_eq!(p as usize % 16, 0);
    unsafe {
        crate::free(p);
    }
}
