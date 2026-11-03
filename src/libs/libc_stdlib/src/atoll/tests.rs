// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::atoll;
use ::sysapi::ffi::c_char;

#[test]
fn positive_value() {
    let s = b"123\0";
    assert_eq!(unsafe { atoll(s.as_ptr().cast::<c_char>()) }, 123);
}

#[test]
fn negative_value() {
    let s = b"-456\0";
    assert_eq!(unsafe { atoll(s.as_ptr().cast::<c_char>()) }, -456);
}

#[test]
fn large_value() {
    let s = b"1000000000000\0";
    assert_eq!(unsafe { atoll(s.as_ptr().cast::<c_char>()) }, 1_000_000_000_000);
}
