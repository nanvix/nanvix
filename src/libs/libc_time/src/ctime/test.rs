// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ctime;
use ::sysapi::sys_types::time_t;

#[test]
fn test_ctime_returns_non_null() {
    let t: time_t = 0;
    let ret: *mut i8 = unsafe { ctime(&t) };
    assert!(!ret.is_null());
}

#[test]
fn test_ctime_null_input() {
    let ret: *mut i8 = unsafe { ctime(core::ptr::null()) };
    assert!(ret.is_null());
}
