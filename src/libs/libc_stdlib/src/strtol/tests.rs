// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strtol;
use crate::set_errno;
use ::sysapi::{
    errno::ERANGE,
    ffi::{
        c_char,
        c_int,
        c_long,
    },
};

fn get_errno() -> c_int {
    unsafe { *sysapi::errno::__errno_location() }
}

#[test]
fn decimal() {
    let s = b"123\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, 123);
}

#[test]
fn negative_decimal() {
    let s = b"-456\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, -456);
}

#[test]
fn hex_with_prefix() {
    let s = b"0xff\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 16) };
    assert_eq!(result, 255);
}

#[test]
fn octal_with_prefix() {
    let s = b"077\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 8) };
    assert_eq!(result, 63);
}

#[test]
fn base_zero_auto_detect_hex() {
    let s = b"0x1a\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 0) };
    assert_eq!(result, 26);
}

#[test]
fn base_zero_auto_detect_octal() {
    let s = b"077\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 0) };
    assert_eq!(result, 63);
}

#[test]
fn base_zero_auto_detect_decimal() {
    let s = b"123\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 0) };
    assert_eq!(result, 123);
}

#[test]
fn overflow_positive() {
    set_errno(0);
    // Value exceeds 64-bit `c_long` (tests run on the 64-bit host), so it overflows on every
    // supported target width.
    let s = b"9999999999999999999999999\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_long::MAX);
    assert_eq!(get_errno(), ERANGE);
}

#[test]
fn overflow_negative() {
    set_errno(0);
    let s = b"-9999999999999999999999999\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_long::MIN);
    assert_eq!(get_errno(), ERANGE);
}

#[test]
fn endptr_set() {
    let s = b"123abc\0";
    let mut end: *mut c_char = core::ptr::null_mut();
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), &mut end, 10) };
    assert_eq!(result, 123);
    assert!(!end.is_null());
    assert_eq!(unsafe { *end } as u8, b'a');
}

#[test]
fn invalid_base() {
    set_errno(0);
    let s = b"123\0";
    let result = unsafe { strtol(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 1) };
    assert_eq!(result, 0);
}
