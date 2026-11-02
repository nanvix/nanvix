// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wmemchr_finds_first_match() {
    let buf: [wchar_t; 4] = [0x41, 0x42, 0x41, 0x43];
    let ret: *mut wchar_t = unsafe { wmemchr(buf.as_ptr(), 0x41, 4) };
    assert_eq!(ret.cast_const(), buf.as_ptr());
}

#[test]
fn test_wmemchr_finds_match_after_first() {
    let buf: [wchar_t; 4] = [0x41, 0x42, 0x43, 0x44];
    let ret: *mut wchar_t = unsafe { wmemchr(buf.as_ptr(), 0x43, 4) };
    assert_eq!(ret.cast_const(), unsafe { buf.as_ptr().add(2) });
}

#[test]
fn test_wmemchr_returns_null_when_absent() {
    let buf: [wchar_t; 3] = [0x41, 0x42, 0x43];
    let ret: *mut wchar_t = unsafe { wmemchr(buf.as_ptr(), 0x44, 3) };
    assert!(ret.is_null());
}

#[test]
fn test_wmemchr_zero_count() {
    let buf: [wchar_t; 1] = [0x41];
    let ret: *mut wchar_t = unsafe { wmemchr(buf.as_ptr(), 0x41, 0) };
    assert!(ret.is_null());
}
