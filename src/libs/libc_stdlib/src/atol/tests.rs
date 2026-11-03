// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::atol;
use ::sysapi::ffi::c_char;

#[test]
fn positive_value() {
    let s = b"123\0";
    assert_eq!(unsafe { atol(s.as_ptr().cast::<c_char>()) }, 123);
}

#[test]
fn negative_value() {
    let s = b"-456\0";
    assert_eq!(unsafe { atol(s.as_ptr().cast::<c_char>()) }, -456);
}
