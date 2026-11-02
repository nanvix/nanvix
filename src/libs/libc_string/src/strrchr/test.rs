// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strrchr;
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
fn test_strrchr_multiple_occurrences() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strrchr(s.as_ptr(), b'l' as c_int) };
    assert!(!ret.is_null(), "strrchr should find 'l'");
    let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
    assert_eq!(offset, 3, "last 'l' should be at index 3");
}

#[test]
fn test_strrchr_single_occurrence() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strrchr(s.as_ptr(), b'h' as c_int) };
    assert!(!ret.is_null(), "strrchr should find 'h'");
    let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
    assert_eq!(offset, 0, "'h' should be at index 0");
}

#[test]
fn test_strrchr_not_found() {
    let s: Vec<c_char> = make_c_string(b"hello");
    let ret: *mut c_char = unsafe { strrchr(s.as_ptr(), b'z' as c_int) };
    assert!(ret.is_null(), "strrchr should return null for char not in string");
}
