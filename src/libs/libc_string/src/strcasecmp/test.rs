// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strcasecmp;
use ::std::vec::Vec;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes
        .iter()
        .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
        .collect();
    v.push(0 as c_char);
    v
}

#[test]
fn test_strcasecmp_equal_ignoring_case() {
    let s1: Vec<c_char> = make_c_string(b"Hello");
    let s2: Vec<c_char> = make_c_string(b"hELLO");
    let ret: c_int = unsafe { strcasecmp(s1.as_ptr(), s2.as_ptr()) };
    assert_eq!(ret, 0, "strcasecmp should ignore case for equal strings");
}

#[test]
fn test_strcasecmp_less() {
    let s1: Vec<c_char> = make_c_string(b"abc");
    let s2: Vec<c_char> = make_c_string(b"abz");
    let ret: c_int = unsafe { strcasecmp(s1.as_ptr(), s2.as_ptr()) };
    assert!(ret < 0, "strcasecmp should return negative when s1 < s2");
}

#[test]
fn test_strcasecmp_greater_case_insensitive() {
    // Case-folded 'Z' (0x7A) is greater than 'c' (0x63).
    let s1: Vec<c_char> = make_c_string(b"abZ");
    let s2: Vec<c_char> = make_c_string(b"abc");
    let ret: c_int = unsafe { strcasecmp(s1.as_ptr(), s2.as_ptr()) };
    assert!(ret > 0, "strcasecmp should compare case-insensitively");
}

#[test]
fn test_strcasecmp_empty_strings() {
    let s1: Vec<c_char> = make_c_string(b"");
    let s2: Vec<c_char> = make_c_string(b"");
    let ret: c_int = unsafe { strcasecmp(s1.as_ptr(), s2.as_ptr()) };
    assert_eq!(ret, 0, "two empty strings should be equal");
}
