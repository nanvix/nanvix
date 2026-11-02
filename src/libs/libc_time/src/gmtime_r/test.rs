// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::gmtime_r;
use crate::tm_struct::tm;
use ::sysapi::sys_types::time_t;

#[test]
fn test_gmtime_r_epoch() {
    // 1970-01-01 00:00:00 UTC.
    let t: time_t = 0;
    let mut result: tm = tm::new();
    let ret: *mut tm = unsafe { gmtime_r(&t, &mut result) };
    assert!(!ret.is_null());
    assert_eq!(result.tm_sec, 0);
    assert_eq!(result.tm_min, 0);
    assert_eq!(result.tm_hour, 0);
    assert_eq!(result.tm_mday, 1);
    assert_eq!(result.tm_mon, 0);
    assert_eq!(result.tm_year, 70);
    assert_eq!(result.tm_wday, 4); // Thursday.
    assert_eq!(result.tm_yday, 0);
}

#[test]
fn test_gmtime_r_known_date() {
    // 2000-01-01 00:00:00 UTC = 946684800 seconds since epoch.
    let t: time_t = 946_684_800;
    let mut result: tm = tm::new();
    let ret: *mut tm = unsafe { gmtime_r(&t, &mut result) };
    assert!(!ret.is_null());
    assert_eq!(result.tm_sec, 0);
    assert_eq!(result.tm_min, 0);
    assert_eq!(result.tm_hour, 0);
    assert_eq!(result.tm_mday, 1);
    assert_eq!(result.tm_mon, 0); // January.
    assert_eq!(result.tm_year, 100); // 2000 - 1900.
    assert_eq!(result.tm_wday, 6); // Saturday.
    assert_eq!(result.tm_yday, 0);
}

#[test]
fn test_gmtime_r_leap_year() {
    // 2000-02-29 12:00:00 UTC = 951825600 seconds since epoch.
    let t: time_t = 951_825_600;
    let mut result: tm = tm::new();
    let ret: *mut tm = unsafe { gmtime_r(&t, &mut result) };
    assert!(!ret.is_null());
    assert_eq!(result.tm_sec, 0);
    assert_eq!(result.tm_min, 0);
    assert_eq!(result.tm_hour, 12);
    assert_eq!(result.tm_mday, 29);
    assert_eq!(result.tm_mon, 1); // February.
    assert_eq!(result.tm_year, 100); // 2000 - 1900.
    assert_eq!(result.tm_yday, 59); // Day 60, zero-indexed = 59.
}

#[test]
fn test_gmtime_r_null_timep() {
    let mut result: tm = tm::new();
    let ret: *mut tm = unsafe { gmtime_r(core::ptr::null(), &mut result) };
    assert!(ret.is_null());
}

#[test]
fn test_gmtime_r_null_result() {
    let t: time_t = 0;
    let ret: *mut tm = unsafe { gmtime_r(&t, core::ptr::null_mut()) };
    assert!(ret.is_null());
}

#[test]
fn test_gmtime_r_year_out_of_range() {
    // Extreme calendar times yield a year that does not fit `tm_year` (a `c_int`); these must
    // be rejected in O(1) rather than looping or silently wrapping the year.
    let mut result: tm = tm::new();
    assert!(unsafe { gmtime_r(&time_t::MAX, &mut result) }.is_null());
    assert!(unsafe { gmtime_r(&time_t::MIN, &mut result) }.is_null());
}

#[test]
fn test_gmtime_r_specific_time() {
    // 1993-06-30 21:49:08 UTC = 741476948 seconds since epoch.
    let t: time_t = 741_476_948;
    let mut result: tm = tm::new();
    let ret: *mut tm = unsafe { gmtime_r(&t, &mut result) };
    assert!(!ret.is_null());
    assert_eq!(result.tm_sec, 8);
    assert_eq!(result.tm_min, 49);
    assert_eq!(result.tm_hour, 21);
    assert_eq!(result.tm_mday, 30);
    assert_eq!(result.tm_mon, 5); // June (0-indexed).
    assert_eq!(result.tm_year, 93); // 1993 - 1900.
    assert_eq!(result.tm_wday, 3); // Wednesday.
}
