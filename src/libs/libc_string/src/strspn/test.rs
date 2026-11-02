// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strspn;
use ::std::vec::Vec;
use ::sysapi::{
    ffi::c_char,
    sys_types::c_size_t,
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
fn test_strspn_all_match() {
    let s: Vec<c_char> = make_c_string(b"aabbcc");
    let accept: Vec<c_char> = make_c_string(b"abc");
    let ret: c_size_t = unsafe { strspn(s.as_ptr(), accept.as_ptr()) };
    assert_eq!(ret as usize, 6, "all chars are in accept");
}

#[test]
fn test_strspn_none_match() {
    let s: Vec<c_char> = make_c_string(b"xyz");
    let accept: Vec<c_char> = make_c_string(b"abc");
    let ret: c_size_t = unsafe { strspn(s.as_ptr(), accept.as_ptr()) };
    assert_eq!(ret as usize, 0, "no chars match");
}

#[test]
fn test_strspn_partial_match() {
    let s: Vec<c_char> = make_c_string(b"aabxyz");
    let accept: Vec<c_char> = make_c_string(b"ab");
    let ret: c_size_t = unsafe { strspn(s.as_ptr(), accept.as_ptr()) };
    assert_eq!(ret as usize, 3, "first 3 chars match");
}
