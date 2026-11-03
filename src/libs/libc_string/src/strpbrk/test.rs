// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strpbrk;
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
fn test_strpbrk_found() {
    let s: Vec<c_char> = make_c_string(b"hello world");
    let accept: Vec<c_char> = make_c_string(b"ow");
    let ret: *mut c_char = unsafe { strpbrk(s.as_ptr(), accept.as_ptr()) };
    assert!(!ret.is_null(), "strpbrk should find a match");
    let offset: usize =
        usize::try_from(unsafe { ret.offset_from(s.as_ptr()) }).expect("offset is non-negative");
    assert_eq!(offset, 4, "first 'o' is at index 4");
}

#[test]
fn test_strpbrk_not_found() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let accept: Vec<c_char> = make_c_string(b"xyz");
    let ret: *mut c_char = unsafe { strpbrk(s.as_ptr(), accept.as_ptr()) };
    assert!(ret.is_null(), "strpbrk should return null when no match");
}

#[test]
fn test_strpbrk_empty_accept() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let accept: Vec<c_char> = make_c_string(b"");
    let ret: *mut c_char = unsafe { strpbrk(s.as_ptr(), accept.as_ptr()) };
    assert!(ret.is_null(), "strpbrk should return null with empty accept");
}
