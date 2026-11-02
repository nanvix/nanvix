// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::localtime_r;
use crate::tm_struct::tm;
use ::sysapi::sys_types::time_t;

#[test]
fn test_localtime_r_epoch() {
    let t: time_t = 0;
    let mut result: tm = tm::new();
    let ret: *mut tm = unsafe { localtime_r(&t, &mut result) };
    assert!(!ret.is_null());
    assert_eq!(result.tm_year, 70);
    assert_eq!(result.tm_mon, 0);
    assert_eq!(result.tm_mday, 1);
}

#[test]
fn test_localtime_r_known_date() {
    // 2000-01-01 00:00:00 UTC.
    let t: time_t = 946_684_800;
    let mut result: tm = tm::new();
    let ret: *mut tm = unsafe { localtime_r(&t, &mut result) };
    assert!(!ret.is_null());
    assert_eq!(result.tm_year, 100);
    assert_eq!(result.tm_mon, 0);
    assert_eq!(result.tm_mday, 1);
}

#[test]
fn test_localtime_r_null_pointers() {
    let t: time_t = 0;
    let mut result: tm = tm::new();
    assert!(unsafe { localtime_r(core::ptr::null(), &mut result) }.is_null());
    assert!(unsafe { localtime_r(&t, core::ptr::null_mut()) }.is_null());
}
