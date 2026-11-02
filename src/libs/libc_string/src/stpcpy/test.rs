// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::stpcpy;
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
fn test_stpcpy_copies_string() {
    let src: Vec<c_char> = make_c_string(b"hello");
    let mut dest: Vec<c_char> = vec![0 as c_char; src.len()];
    unsafe { stpcpy(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(dest, src, "stpcpy should copy the whole string including the terminator");
}

#[test]
fn test_stpcpy_returns_end_pointer() {
    let src: Vec<c_char> = make_c_string(b"hello");
    let mut dest: Vec<c_char> = vec![0 as c_char; src.len()];
    let ret: *mut c_char = unsafe { stpcpy(dest.as_mut_ptr(), src.as_ptr()) };
    let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
    assert_eq!(offset, 5, "stpcpy should return a pointer to the terminating null byte");
    assert_eq!(unsafe { *ret }, 0 as c_char, "the returned pointer must point at a null byte");
}

#[test]
fn test_stpcpy_empty_string() {
    let src: Vec<c_char> = make_c_string(b"");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 1];
    let ret: *mut c_char = unsafe { stpcpy(dest.as_mut_ptr(), src.as_ptr()) };
    let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
    assert_eq!(offset, 0, "stpcpy of an empty string should return the destination start");
    assert_eq!(dest[0], 0 as c_char, "stpcpy should write the terminating null byte");
}
