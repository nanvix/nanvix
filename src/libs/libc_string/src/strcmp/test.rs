// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strcmp;
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
fn test_strcmp_equal() {
    let s1: Vec<c_char> = make_c_string(b"hello");
    let s2: Vec<c_char> = make_c_string(b"hello");
    let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
    assert_eq!(ret, 0, "strcmp should return 0 for equal strings");
}

#[test]
fn test_strcmp_first_less() {
    let s1: Vec<c_char> = make_c_string(b"abc");
    let s2: Vec<c_char> = make_c_string(b"abz");
    let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
    assert!(ret < 0, "strcmp should return negative when s1 < s2");
}

#[test]
fn test_strcmp_first_greater() {
    let s1: Vec<c_char> = make_c_string(b"abz");
    let s2: Vec<c_char> = make_c_string(b"abc");
    let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
    assert!(ret > 0, "strcmp should return positive when s1 > s2");
}

#[test]
fn test_strcmp_empty_strings() {
    let s1: Vec<c_char> = make_c_string(b"");
    let s2: Vec<c_char> = make_c_string(b"");
    let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
    assert_eq!(ret, 0, "strcmp should return 0 for two empty strings");
}

#[test]
fn test_strcmp_one_empty() {
    let s1: Vec<c_char> = make_c_string(b"");
    let s2: Vec<c_char> = make_c_string(b"a");
    let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
    assert!(ret < 0, "strcmp should return negative when s1 is empty and s2 is not");
}
