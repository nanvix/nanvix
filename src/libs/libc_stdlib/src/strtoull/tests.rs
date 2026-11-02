// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strtoull;
use crate::set_errno;
use ::sysapi::{
    errno::ERANGE,
    ffi::{
        c_char,
        c_int,
        c_ulonglong,
    },
};

fn get_errno() -> c_int {
    unsafe { *sysapi::errno::__errno_location() }
}

#[test]
fn basic_decimal() {
    let s = b"123\0";
    let result = unsafe { strtoull(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, 123);
}

#[test]
fn large_value() {
    let s = b"18446744073709551615\0"; // u64::MAX
    let result = unsafe { strtoull(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_ulonglong::MAX);
}

#[test]
fn overflow() {
    set_errno(0);
    let s = b"18446744073709551616\0"; // u64::MAX + 1
    let result = unsafe { strtoull(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
    assert_eq!(result, c_ulonglong::MAX);
    assert_eq!(get_errno(), ERANGE);
}
