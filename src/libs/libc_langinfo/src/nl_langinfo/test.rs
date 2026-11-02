// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    nl_item,
    nl_langinfo,
};
use ::sysapi::ffi::c_char;

// Item identifiers mirrored from the implementation for test readability.
const CODESET: nl_item = 0;
const D_FMT: nl_item = 2;
const RADIXCHAR: nl_item = 45;
const THOUSEP: nl_item = 46;
const DAY_1: nl_item = 7;
const DAY_7: nl_item = 13;
const ABDAY_1: nl_item = 14;
const MON_1: nl_item = 21;
const MON_12: nl_item = 32;
const ABMON_1: nl_item = 33;

/// Compares a C string returned by `nl_langinfo` against an expected byte slice (no NUL).
fn c_str_eq(ptr: *mut c_char, expected: &[u8]) -> bool {
    assert!(!ptr.is_null());
    for (i, &byte) in expected.iter().enumerate() {
        let want: c_char = c_char::try_from(byte).expect("ASCII fits in c_char");
        if unsafe { *ptr.add(i) } != want {
            return false;
        }
    }
    // The returned string must terminate right after the expected bytes.
    unsafe { *ptr.add(expected.len()) == 0 }
}

#[test]
fn test_codeset_is_iso_8859_1() {
    assert!(c_str_eq(nl_langinfo(CODESET), b"ISO-8859-1"));
}

#[test]
fn test_date_format() {
    assert!(c_str_eq(nl_langinfo(D_FMT), b"%m/%d/%y"));
}

#[test]
fn test_radixchar_and_thousep() {
    assert!(c_str_eq(nl_langinfo(RADIXCHAR), b"."));
    assert!(c_str_eq(nl_langinfo(THOUSEP), b""));
}

#[test]
fn test_day_names() {
    assert!(c_str_eq(nl_langinfo(DAY_1), b"Sunday"));
    assert!(c_str_eq(nl_langinfo(DAY_7), b"Saturday"));
    assert!(c_str_eq(nl_langinfo(ABDAY_1), b"Sun"));
}

#[test]
fn test_month_names() {
    assert!(c_str_eq(nl_langinfo(MON_1), b"January"));
    assert!(c_str_eq(nl_langinfo(MON_12), b"December"));
    assert!(c_str_eq(nl_langinfo(ABMON_1), b"Jan"));
}

#[test]
fn test_unknown_item_is_empty() {
    assert!(c_str_eq(nl_langinfo(9999), b""));
    assert!(c_str_eq(nl_langinfo(-1), b""));
}
