// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strchrnul;
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
fn test_strchrnul_found() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strchrnul(s.as_ptr(), b'l' as c_int) };
    let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
    assert_eq!(offset, 2, "strchrnul should find the first 'l' at index 2");
}

#[test]
fn test_strchrnul_not_found_returns_terminator() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strchrnul(s.as_ptr(), b'z' as c_int) };
    let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
    assert_eq!(offset, 5, "strchrnul should return the terminator when char is not found");
    assert_eq!(unsafe { *ret }, 0 as c_char, "the returned pointer must point at a null byte");
}

#[test]
fn test_strchrnul_find_null_terminator() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strchrnul(s.as_ptr(), 0) };
    let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
    assert_eq!(offset, 5, "strchrnul(.., 0) should return the terminator");
}

#[test]
fn test_strchrnul_empty_string() {
    let s: Vec<c_char> = make_c_string(b"");
    let ret: *mut c_char = unsafe { strchrnul(s.as_ptr(), b'a' as c_int) };
    let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
    assert_eq!(offset, 0, "strchrnul of an empty string should return the start (terminator)");
}
