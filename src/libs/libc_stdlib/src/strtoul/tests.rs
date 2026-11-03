// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strtoul;
use crate::set_errno;
use ::sysapi::{
    errno::ERANGE,
    ffi::{
        c_char,
        c_int,
        c_ulong,
    },
};

fn get_errno() -> c_int {
    unsafe { *sysapi::errno::__errno_location() }
}

#[test]
fn basic_decimal() {
    let s = b"123\0";
    let result = unsafe { strtoul(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, 123);
}

#[test]
fn hex_value() {
    let s = b"0xff\0";
    let result = unsafe { strtoul(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 16) };
    assert_eq!(result, 255);
}

#[test]
fn overflow() {
    set_errno(0);
    // Value exceeds 64-bit `c_ulong` (tests run on the 64-bit host), so it overflows on every
    // supported target width.
    let s = b"9999999999999999999999999\0";
    let result = unsafe { strtoul(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_ulong::MAX);
    assert_eq!(get_errno(), ERANGE);
}

#[test]
fn negative_wraps() {
    let s = b"-1\0";
    let result = unsafe { strtoul(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_ulong::MAX);
}
