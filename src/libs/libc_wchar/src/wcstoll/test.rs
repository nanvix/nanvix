// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcstoll_large_value() {
    // 10_000_000_000 exceeds 32 bits, exercising long long width.
    let s: [wchar_t; 12] = [
        0x31, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0,
    ];
    let v: c_longlong = unsafe { wcstoll(s.as_ptr(), core::ptr::null_mut(), 10) };
    assert_eq!(v, 10_000_000_000);
}

#[test]
fn test_wcstoll_negative() {
    // "-75" base 10 -> -75.
    let s: [wchar_t; 4] = [0x2D, 0x37, 0x35, 0];
    let v: c_longlong = unsafe { wcstoll(s.as_ptr(), core::ptr::null_mut(), 10) };
    assert_eq!(v, -75);
}
