// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcstol_decimal() {
    // "123" base 10 -> 123; the end pointer lands on the terminator.
    let s: [wchar_t; 4] = [0x31, 0x32, 0x33, 0];
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let v: c_long = unsafe { wcstol(s.as_ptr(), &mut end, 10) };
    assert_eq!(v, 123);
    assert_eq!(unsafe { *end }, 0);
}

#[test]
fn test_wcstol_hex() {
    // "ff" base 16 -> 255.
    let s: [wchar_t; 3] = [0x66, 0x66, 0];
    let v: c_long = unsafe { wcstol(s.as_ptr(), core::ptr::null_mut(), 16) };
    assert_eq!(v, 255);
}

#[test]
fn test_wcstol_endptr_stops_at_non_digit() {
    // "42xyz" parses 42 and leaves the end pointer at 'x' (index 2). The narrow byte offset is
    // mapped back to a wide-character offset, which is width-independent.
    let s: [wchar_t; 6] = [0x34, 0x32, 0x78, 0x79, 0x7A, 0];
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let v: c_long = unsafe { wcstol(s.as_ptr(), &mut end, 10) };
    assert_eq!(v, 42);
    assert_eq!(end.cast_const(), unsafe { s.as_ptr().add(2) });
}

#[test]
fn test_wcstol_endptr_after_long_subject_sequence() {
    let mut s: [wchar_t; 82] = [0; 82];
    for c in s.iter_mut().take(80) {
        *c = 0x31;
    }
    s[80] = 0x78;
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let _v: c_long = unsafe { wcstol(s.as_ptr(), &mut end, 10) };
    assert_eq!(end.cast_const(), unsafe { s.as_ptr().add(80) });
}
