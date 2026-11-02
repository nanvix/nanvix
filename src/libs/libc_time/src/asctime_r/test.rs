// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::asctime_r;
use crate::tm_struct::tm;
use ::sysapi::ffi::c_char;

fn buf_to_string(buf: &[c_char; 26]) -> ::std::string::String {
    let bytes: ::std::vec::Vec<u8> = buf
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    ::std::string::String::from_utf8(bytes).expect("valid UTF-8")
}

#[test]
fn test_asctime_r_known_date() {
    // Wednesday, June 30, 1993 21:49:08.
    let t: tm = tm {
        tm_sec: 8,
        tm_min: 49,
        tm_hour: 21,
        tm_mday: 30,
        tm_mon: 5,
        tm_year: 93,
        tm_wday: 3,
        tm_yday: 180,
        tm_isdst: 0,
        tm_gmtoff: 0,
    };
    let mut buf: [c_char; 26] = [0; 26];
    let ret: *mut c_char = unsafe { asctime_r(&t, buf.as_mut_ptr()) };
    assert!(!ret.is_null());
    assert_eq!(buf_to_string(&buf), "Wed Jun 30 21:49:08 1993\n");
}

#[test]
fn test_asctime_r_epoch() {
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
    let mut buf: [c_char; 26] = [0; 26];
    let ret: *mut c_char = unsafe { asctime_r(&t, buf.as_mut_ptr()) };
    assert!(!ret.is_null());
    assert_eq!(buf_to_string(&buf), "Thu Jan  1 00:00:00 1970\n");
}

#[test]
fn test_asctime_r_null_pointers() {
    let t: tm = tm::new();
    let mut buf: [c_char; 26] = [0; 26];
    assert!(unsafe { asctime_r(core::ptr::null(), buf.as_mut_ptr()) }.is_null());
    assert!(unsafe { asctime_r(&t, core::ptr::null_mut()) }.is_null());
}

#[test]
fn test_asctime_r_year_out_of_range() {
    // Years that do not fit the fixed 4-digit field must be rejected rather than overflowing
    // the 26-byte buffer.
    let mut t: tm = tm::new();
    t.tm_mday = 1;
    t.tm_wday = 4;

    t.tm_year = -2000; // Year -100 (before year 0).
    let mut buf: [c_char; 26] = [0; 26];
    assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());

    t.tm_year = 9000; // Year 10900 (more than 4 digits).
    assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());
}

#[test]
fn test_asctime_r_invalid_fields() {
    // Out-of-range numeric fields must be rejected rather than producing malformed output.
    let base: tm = tm {
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
    let mut buf: [c_char; 26] = [0; 26];

    let mut t: tm = base;
    t.tm_mday = 0;
    assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());

    let mut t: tm = base;
    t.tm_hour = 24;
    assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());

    let mut t: tm = base;
    t.tm_min = -1;
    assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());

    let mut t: tm = base;
    t.tm_sec = 61;
    assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());
}
