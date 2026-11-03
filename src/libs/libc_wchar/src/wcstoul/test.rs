// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcstoul_decimal() {
    // "123" base 10 -> 123.
    let s: [wchar_t; 4] = [0x31, 0x32, 0x33, 0];
    let v: c_ulong = unsafe { wcstoul(s.as_ptr(), core::ptr::null_mut(), 10) };
    assert_eq!(v, 123);
}

#[test]
fn test_wcstoul_hex() {
    // "ff" base 16 -> 255; the end pointer lands on the terminator.
    let s: [wchar_t; 3] = [0x66, 0x66, 0];
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let v: c_ulong = unsafe { wcstoul(s.as_ptr(), &mut end, 16) };
    assert_eq!(v, 255);
    assert_eq!(unsafe { *end }, 0);
}

#[test]
fn test_wcstoul_endptr_stops_at_non_digit() {
    // "99.5" parses 99 and leaves the end pointer at '.' (index 2).
    let s: [wchar_t; 5] = [0x39, 0x39, 0x2E, 0x35, 0];
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let v: c_ulong = unsafe { wcstoul(s.as_ptr(), &mut end, 10) };
    assert_eq!(v, 99);
    assert_eq!(end.cast_const(), unsafe { s.as_ptr().add(2) });
}
