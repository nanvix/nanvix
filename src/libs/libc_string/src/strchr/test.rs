// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strchr;
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
fn test_strchr_found() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strchr(s.as_ptr(), b'l' as c_int) };
    assert!(!ret.is_null(), "strchr should find 'l'");
    let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
    assert_eq!(offset, 2, "first 'l' should be at index 2");
}

#[test]
fn test_strchr_not_found() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strchr(s.as_ptr(), b'z' as c_int) };
    assert!(ret.is_null(), "strchr should return null for char not in string");
}

#[test]
fn test_strchr_find_null_terminator() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strchr(s.as_ptr(), 0) };
    assert!(!ret.is_null(), "strchr should find null terminator");
    let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
    assert_eq!(offset, 5, "null terminator should be at index 5");
}

#[test]
fn test_strchr_empty_string() {
    let s: Vec<c_char> = make_c_string(b"");
    let ret: *mut c_char = unsafe { strchr(s.as_ptr(), b'a' as c_int) };
    assert!(ret.is_null(), "strchr should return null for empty string");
}
