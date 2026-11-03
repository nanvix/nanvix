// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strcasestr;
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
fn test_strcasestr_found_case_insensitive() {
    let haystack: Vec<c_char> = make_c_string(b"Hello, World");
    let needle: Vec<c_char> = make_c_string(b"world");
    let ret: *mut c_char = unsafe { strcasestr(haystack.as_ptr(), needle.as_ptr()) };
    assert!(!ret.is_null(), "strcasestr should find 'world' ignoring case");
    let offset: usize = unsafe { ret.offset_from(haystack.as_ptr()) } as usize;
    assert_eq!(offset, 7, "match should start at index 7");
}

#[test]
fn test_strcasestr_not_found() {
    let haystack: Vec<c_char> = make_c_string(b"Hello");
    let needle: Vec<c_char> = make_c_string(b"xyz");
    let ret: *mut c_char = unsafe { strcasestr(haystack.as_ptr(), needle.as_ptr()) };
    assert!(ret.is_null(), "strcasestr should return null when the needle is not found");
}

#[test]
fn test_strcasestr_empty_needle() {
    let haystack: Vec<c_char> = make_c_string(b"Hello");
    let needle: Vec<c_char> = make_c_string(b"");
    let ret: *mut c_char = unsafe { strcasestr(haystack.as_ptr(), needle.as_ptr()) };
    assert_eq!(ret, haystack.as_ptr().cast_mut(), "empty needle should return haystack");
}

#[test]
fn test_strcasestr_prefix_match() {
    let haystack: Vec<c_char> = make_c_string(b"ABCabc");
    let needle: Vec<c_char> = make_c_string(b"abc");
    let ret: *mut c_char = unsafe { strcasestr(haystack.as_ptr(), needle.as_ptr()) };
    let offset: usize = unsafe { ret.offset_from(haystack.as_ptr()) } as usize;
    assert_eq!(offset, 0, "strcasestr should match the first (case-folded) occurrence");
}
