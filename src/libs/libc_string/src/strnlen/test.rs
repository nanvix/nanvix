// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strnlen;
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
fn test_strnlen_shorter_than_maxlen() {
    let buf: Vec<c_char> = make_c_string(b"hello");
    let len: usize = unsafe { strnlen(buf.as_ptr(), 100) as usize };
    assert_eq!(len, 5, "strnlen should return actual length when shorter than maxlen");
}

#[test]
fn test_strnlen_longer_than_maxlen() {
    let buf: Vec<c_char> = make_c_string(b"hello world");
    let len: usize = unsafe { strnlen(buf.as_ptr(), 5) as usize };
    assert_eq!(len, 5, "strnlen should return maxlen when string is longer");
}

#[test]
fn test_strnlen_empty_string() {
    let buf: Vec<c_char> = make_c_string(b"");
    let len: usize = unsafe { strnlen(buf.as_ptr(), 100) as usize };
    assert_eq!(len, 0, "strnlen of empty string should be 0");
}

#[test]
fn test_strnlen_zero_maxlen() {
    let buf: Vec<c_char> = make_c_string(b"hello");
    let len: usize = unsafe { strnlen(buf.as_ptr(), 0) as usize };
    assert_eq!(len, 0, "strnlen with maxlen 0 should return 0");
}

#[test]
fn test_strnlen_exact_maxlen() {
    let buf: Vec<c_char> = make_c_string(b"hello");
    let len: usize = unsafe { strnlen(buf.as_ptr(), 5) as usize };
    assert_eq!(len, 5, "strnlen should return 5 when maxlen equals string length");
}
