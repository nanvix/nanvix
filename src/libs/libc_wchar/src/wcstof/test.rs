// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcstof_simple() {
    // "3.50" parses to 3.5 and leaves the end pointer at the terminator.
    let s: [wchar_t; 5] = [0x33, 0x2E, 0x35, 0x30, 0];
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let v: f32 = unsafe { wcstof(s.as_ptr(), &mut end) };
    assert!((v - 3.5).abs() < 1e-6);
    assert_eq!(unsafe { *end }, 0);
}

#[test]
fn test_wcstof_partial() {
    // "2.5x" parses 2.5 and stops at 'x'.
    let s: [wchar_t; 5] = [0x32, 0x2E, 0x35, 0x78, 0];
    let mut end: *mut wchar_t = core::ptr::null_mut();
    let v: f32 = unsafe { wcstof(s.as_ptr(), &mut end) };
    assert!((v - 2.5).abs() < 1e-6);
    assert_eq!(unsafe { *end }, 0x78);
}

#[test]
fn test_wcstof_null_endptr() {
    // A null end pointer is accepted.
    let s: [wchar_t; 3] = [0x31, 0x30, 0];
    let v: f32 = unsafe { wcstof(s.as_ptr(), core::ptr::null_mut()) };
    assert!((v - 10.0).abs() < 1e-6);
}
