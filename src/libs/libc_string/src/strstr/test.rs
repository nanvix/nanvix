// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strstr;
use ::std::vec::Vec;
use ::sysapi::ffi::c_char;

fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes
        .iter()
        .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
        .collect();
    v.push(0 as c_char);
    v
}

#[test]
fn test_strstr_found() {
    let haystack: Vec<c_char> = make_c_string(b"hello world");
    let needle: Vec<c_char> = make_c_string(b"world");
    let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
    assert!(!ret.is_null(), "strstr should find 'world'");
    let offset: usize = usize::try_from(unsafe { ret.offset_from(haystack.as_ptr()) })
        .expect("offset is non-negative");
    assert_eq!(offset, 6, "'world' starts at index 6");
}

#[test]
fn test_strstr_not_found() {
    let haystack: Vec<c_char> = make_c_string(b"hello world");
    let needle: Vec<c_char> = make_c_string(b"xyz");
    let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
    assert!(ret.is_null(), "strstr should return null when needle not found");
}

#[test]
fn test_strstr_empty_needle() {
    let haystack: Vec<c_char> = make_c_string(b"hello");
    let needle: Vec<c_char> = make_c_string(b"");
    let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
    assert_eq!(ret, haystack.as_ptr().cast_mut(), "empty needle returns haystack");
}

#[test]
fn test_strstr_at_start() {
    let haystack: Vec<c_char> = make_c_string(b"hello");
    let needle: Vec<c_char> = make_c_string(b"hell");
    let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
    assert!(!ret.is_null(), "strstr should find needle at start");
    let offset: usize = usize::try_from(unsafe { ret.offset_from(haystack.as_ptr()) })
        .expect("offset is non-negative");
    assert_eq!(offset, 0, "needle at start should be at index 0");
}

#[test]
fn test_strstr_at_end() {
    let haystack: Vec<c_char> = make_c_string(b"hello");
    let needle: Vec<c_char> = make_c_string(b"llo");
    let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
    assert!(!ret.is_null(), "strstr should find needle at end");
    let offset: usize = usize::try_from(unsafe { ret.offset_from(haystack.as_ptr()) })
        .expect("offset is non-negative");
    assert_eq!(offset, 2, "'llo' starts at index 2");
}
