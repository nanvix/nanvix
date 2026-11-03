// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strncasecmp;
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
fn test_strncasecmp_equal_within_n() {
    let s1: Vec<c_char> = make_c_string(b"Hello");
    let s2: Vec<c_char> = make_c_string(b"hELLO");
    let ret: c_int = unsafe { strncasecmp(s1.as_ptr(), s2.as_ptr(), 5) };
    assert_eq!(ret, 0, "strncasecmp should ignore case within n bytes");
}

#[test]
fn test_strncasecmp_n_limits_comparison() {
    // Strings differ only after the first three bytes; n=3 ignores the rest.
    let s1: Vec<c_char> = make_c_string(b"abcXXX");
    let s2: Vec<c_char> = make_c_string(b"abcYYY");
    let ret: c_int = unsafe { strncasecmp(s1.as_ptr(), s2.as_ptr(), 3) };
    assert_eq!(ret, 0, "strncasecmp should only compare the first n bytes");
}

#[test]
fn test_strncasecmp_differ_within_n() {
    let s1: Vec<c_char> = make_c_string(b"abc");
    let s2: Vec<c_char> = make_c_string(b"abz");
    let ret: c_int = unsafe { strncasecmp(s1.as_ptr(), s2.as_ptr(), 3) };
    assert!(ret < 0, "strncasecmp should return negative when s1 < s2 within n");
}

#[test]
fn test_strncasecmp_zero_n() {
    let s1: Vec<c_char> = make_c_string(b"abc");
    let s2: Vec<c_char> = make_c_string(b"xyz");
    let ret: c_int = unsafe { strncasecmp(s1.as_ptr(), s2.as_ptr(), 0) };
    assert_eq!(ret, 0, "strncasecmp should return 0 when n is 0");
}
