// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strncmp;
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
fn test_strncmp_equal_within_n() {
    let s1: Vec<c_char> = make_c_string(b"hello");
    let s2: Vec<c_char> = make_c_string(b"hello");
    let ret: c_int = unsafe { strncmp(s1.as_ptr(), s2.as_ptr(), 5) };
    assert_eq!(ret, 0, "strncmp should return 0 for equal strings within n");
}

#[test]
fn test_strncmp_differ_within_n() {
    let s1: Vec<c_char> = make_c_string(b"helly");
    let s2: Vec<c_char> = make_c_string(b"hello");
    let ret: c_int = unsafe { strncmp(s1.as_ptr(), s2.as_ptr(), 5) };
    assert!(ret != 0, "strncmp should return non-zero when strings differ within n");
}

#[test]
fn test_strncmp_n_zero() {
    let s1: Vec<c_char> = make_c_string(b"abc");
    let s2: Vec<c_char> = make_c_string(b"xyz");
    let ret: c_int = unsafe { strncmp(s1.as_ptr(), s2.as_ptr(), 0) };
    assert_eq!(ret, 0, "strncmp should return 0 when n is 0");
}

#[test]
fn test_strncmp_n_greater_than_length() {
    let s1: Vec<c_char> = make_c_string(b"ab");
    let s2: Vec<c_char> = make_c_string(b"ab");
    let ret: c_int = unsafe { strncmp(s1.as_ptr(), s2.as_ptr(), 100) };
    assert_eq!(ret, 0, "strncmp should return 0 for equal strings even when n > length");
}
