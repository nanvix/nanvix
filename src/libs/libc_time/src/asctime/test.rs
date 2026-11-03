// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::asctime;
use crate::tm_struct::tm;

#[test]
fn test_asctime_returns_non_null() {
    let t: tm = tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 1,
        tm_mon: 0,
        tm_year: 70,
        tm_wday: 4,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
    };
    let ret: *mut i8 = unsafe { asctime(&t) };
    assert!(!ret.is_null());
}

#[test]
fn test_asctime_null_input() {
    let ret: *mut i8 = unsafe { asctime(core::ptr::null()) };
    assert!(ret.is_null());
}
