// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::atoi;
use ::sysapi::ffi::c_char;

#[test]
fn positive_value() {
    let s = b"123\0";
    assert_eq!(unsafe { atoi(s.as_ptr().cast::<c_char>()) }, 123);
}

#[test]
fn negative_value() {
    let s = b"-456\0";
    assert_eq!(unsafe { atoi(s.as_ptr().cast::<c_char>()) }, -456);
}

#[test]
fn with_whitespace_and_sign() {
    let s = b"  +789\0";
    assert_eq!(unsafe { atoi(s.as_ptr().cast::<c_char>()) }, 789);
}

#[test]
fn non_numeric() {
    let s = b"abc\0";
    assert_eq!(unsafe { atoi(s.as_ptr().cast::<c_char>()) }, 0);
}

#[test]
fn overflow_wraps() {
    let s = b"99999999999\0";
    // Overflow is undefined in C; just verify no crash.
    let _ = unsafe { atoi(s.as_ptr().cast::<c_char>()) };
}
