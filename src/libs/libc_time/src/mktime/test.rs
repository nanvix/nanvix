// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::mktime;
use crate::tm_struct::tm;
use ::sysapi::sys_types::time_t;

#[test]
fn test_mktime_epoch() {
    let mut t: tm = tm::new();
    t.tm_year = 70;
    t.tm_mon = 0;
    t.tm_mday = 1;
    let result: time_t = unsafe { mktime(&mut t) };
    assert_eq!(result, 0);
}

#[test]
fn test_mktime_known_date() {
    // 2000-01-01 00:00:00 UTC.
    let mut t: tm = tm::new();
    t.tm_year = 100;
    t.tm_mon = 0;
    t.tm_mday = 1;
    let result: time_t = unsafe { mktime(&mut t) };
    assert_eq!(result, 946_684_800);
}

#[test]
fn test_mktime_round_trip() {
    let original: time_t = 741_476_948;
    let mut broken: tm = tm::new();
    unsafe { crate::gmtime_r::gmtime_r(&original, &mut broken) };
    let result: time_t = unsafe { mktime(&mut broken) };
    assert_eq!(result, original);
}

#[test]
fn test_mktime_normalizes() {
    // 1970-01-01 00:00:90 (90 seconds should normalize to 00:01:30).
    let mut t: tm = tm::new();
    t.tm_year = 70;
    t.tm_mon = 0;
    t.tm_mday = 1;
    t.tm_sec = 90;
    let result: time_t = unsafe { mktime(&mut t) };
    assert_eq!(result, 90);
    assert_eq!(t.tm_min, 1);
    assert_eq!(t.tm_sec, 30);
}

#[test]
fn test_mktime_null() {
    let result: time_t = unsafe { mktime(core::ptr::null_mut()) };
    assert_eq!(result, -1);
}
