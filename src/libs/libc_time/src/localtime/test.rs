// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::localtime;
use crate::tm_struct::tm;
use ::sysapi::sys_types::time_t;

#[test]
fn test_localtime_returns_non_null() {
    let t: time_t = 0;
    let ret: *mut tm = unsafe { localtime(&t) };
    assert!(!ret.is_null());
    assert_eq!(unsafe { (*ret).tm_year }, 70);
}

#[test]
fn test_localtime_null_input() {
    let ret: *mut tm = unsafe { localtime(core::ptr::null()) };
    assert!(ret.is_null());
}
