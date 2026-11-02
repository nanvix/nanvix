// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strlcpy;
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
fn test_strlcpy_normal() {
    let src: Vec<c_char> = make_c_string(b"hello");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 10];
    let ret: c_size_t = unsafe { strlcpy(dest.as_mut_ptr(), src.as_ptr(), 10) };
    assert_eq!(ret as usize, 5, "strlcpy should return strlen(src)");
    assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
    assert_eq!(dest[5], 0 as c_char);
}

#[test]
fn test_strlcpy_truncation() {
    let src: Vec<c_char> = make_c_string(b"hello world");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 6];
    let ret: c_size_t = unsafe { strlcpy(dest.as_mut_ptr(), src.as_ptr(), 6) };
    assert_eq!(ret as usize, 11, "strlcpy should return full strlen(src)");
    assert_eq!(dest[5], 0 as c_char, "dest should be null-terminated");
    assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
    assert_eq!(dest[4], c_char::try_from(b'o').expect("ASCII fits in c_char"));
}

#[test]
fn test_strlcpy_zero_size() {
    let src: Vec<c_char> = make_c_string(b"hello");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 5];
    let ret: c_size_t = unsafe { strlcpy(dest.as_mut_ptr(), src.as_ptr(), 0) };
    assert_eq!(ret as usize, 5, "strlcpy should return strlen(src) even with size 0");
    assert_eq!(dest[0], 0x7F as c_char, "dest should be unchanged with size 0");
}
