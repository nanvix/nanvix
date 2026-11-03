// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcstod_simple() {
    // "3.50" parses to 3.5 and leaves the end pointer at the terminator.
    let s: [wchar_t; 5] = [0x33, 0x2E, 0x35, 0x30, 0];
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let v: f64 = unsafe { wcstod(s.as_ptr(), &mut end) };
    assert!((v - 3.5).abs() < 1e-9);
    assert_eq!(unsafe { *end }, 0);
}

#[test]
fn test_wcstod_partial() {
    // "2.5x" parses 2.5 and stops at 'x'.
    let s: [wchar_t; 5] = [0x32, 0x2E, 0x35, 0x78, 0];
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let v: f64 = unsafe { wcstod(s.as_ptr(), &mut end) };
    assert!((v - 2.5).abs() < 1e-9);
    assert_eq!(unsafe { *end }, 0x78);
}

#[test]
fn test_wcstod_null_endptr() {
    // A null end pointer is accepted.
    let s: [wchar_t; 3] = [0x31, 0x30, 0];
    let v: f64 = unsafe { wcstod(s.as_ptr(), core::ptr::null_mut()) };
    assert!((v - 10.0).abs() < 1e-9);
}

#[test]
fn test_wcstod_endptr_after_long_subject_sequence() {
    let mut s: [wchar_t; 112] = [0; 112];
    s[0] = 0x31;
    s[1] = 0x2E;
    for c in s.iter_mut().take(110).skip(2) {
        *c = 0x31;
    }
    s[110] = 0x78;
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let _v: f64 = unsafe { wcstod(s.as_ptr(), &mut end) };
    assert_eq!(end.cast_const(), unsafe { s.as_ptr().add(110) });
}
