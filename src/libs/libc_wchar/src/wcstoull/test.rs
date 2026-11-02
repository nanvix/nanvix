// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcstoull_large_value() {
    // 20_000_000_000 exceeds 32 bits, exercising unsigned long long width.
    let s: [wchar_t; 12] = [
        0x32, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0,
    ];
    let v: c_ulonglong = unsafe { wcstoull(s.as_ptr(), core::ptr::null_mut(), 10) };
    assert_eq!(v, 20_000_000_000);
}

#[test]
fn test_wcstoull_hex() {
    // "abc" base 16 -> 2748.
    let s: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let v: c_ulonglong = unsafe { wcstoull(s.as_ptr(), core::ptr::null_mut(), 16) };
    assert_eq!(v, 2748);
}
