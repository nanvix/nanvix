// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strcpy;
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
fn test_strcpy_basic() {
    let src: Vec<c_char> = make_c_string(b"hello");
    let mut dest: Vec<c_char> = vec![0 as c_char; 10];
    let ret: *mut c_char = unsafe { strcpy(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(ret, dest.as_mut_ptr(), "strcpy should return dest");
    assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
    assert_eq!(dest[5], 0 as c_char);
}

#[test]
fn test_strcpy_empty() {
    let src: Vec<c_char> = make_c_string(b"");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 5];
    unsafe { strcpy(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(dest[0], 0 as c_char, "strcpy of empty string should write null terminator");
}
