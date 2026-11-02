// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strcspn;
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
fn test_strcspn_all_rejected() {
    let s: Vec<c_char> = make_c_string(b"abc");
    let reject: Vec<c_char> = make_c_string(b"abc");
    let ret: c_size_t = unsafe { strcspn(s.as_ptr(), reject.as_ptr()) };
    assert_eq!(ret as usize, 0, "first char is rejected");
}

#[test]
fn test_strcspn_none_rejected() {
    let s: Vec<c_char> = make_c_string(b"abc");
    let reject: Vec<c_char> = make_c_string(b"xyz");
    let ret: c_size_t = unsafe { strcspn(s.as_ptr(), reject.as_ptr()) };
    assert_eq!(ret as usize, 3, "no chars are rejected");
}

#[test]
fn test_strcspn_partial() {
    let s: Vec<c_char> = make_c_string(b"abcxyz");
    let reject: Vec<c_char> = make_c_string(b"xyz");
    let ret: c_size_t = unsafe { strcspn(s.as_ptr(), reject.as_ptr()) };
    assert_eq!(ret as usize, 3, "first 3 chars are not rejected");
}
