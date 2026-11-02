// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strtoll;
use crate::set_errno;
use ::sysapi::{
    errno::ERANGE,
    ffi::{
        c_char,
        c_int,
        c_longlong,
    },
};

fn get_errno() -> c_int {
    unsafe { *sysapi::errno::__errno_location() }
}

#[test]
fn decimal() {
    let s = b"123\0";
    let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, 123);
}

#[test]
fn large_positive() {
    let s = b"9223372036854775807\0"; // i64::MAX
    let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_longlong::MAX);
}

#[test]
fn large_negative() {
    let s = b"-9223372036854775808\0"; // i64::MIN
    let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_longlong::MIN);
}

#[test]
fn overflow_positive() {
    set_errno(0);
    let s = b"9223372036854775808\0"; // i64::MAX + 1
    let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_longlong::MAX);
    assert_eq!(get_errno(), ERANGE);
}

#[test]
fn overflow_negative() {
    set_errno(0);
    let s = b"-9223372036854775809\0"; // i64::MIN - 1
    let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_longlong::MIN);
    assert_eq!(get_errno(), ERANGE);
}
