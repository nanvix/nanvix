// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::stpncpy;
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
fn test_stpncpy_copies_and_pads() {
    let src: Vec<c_char> = make_c_string(b"abc");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 6];
    let ret: *mut c_char = unsafe { stpncpy(dest.as_mut_ptr(), src.as_ptr(), 6) };
    let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
    assert_eq!(offset, 3, "stpncpy should return a pointer to the first null byte");
    let mut expected: Vec<c_char> = b"abc"
        .iter()
        .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
        .collect();
    expected.resize(6, 0 as c_char);
    assert_eq!(dest, expected, "stpncpy should pad the remainder with null bytes");
}

#[test]
fn test_stpncpy_truncates_without_terminator() {
    let src: Vec<c_char> = make_c_string(b"abcdef");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 3];
    let ret: *mut c_char = unsafe { stpncpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
    let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
    assert_eq!(offset, 3, "stpncpy should return dest + n when no null byte is written");
    let expected: Vec<c_char> = b"abc"
        .iter()
        .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
        .collect();
    assert_eq!(dest, expected, "stpncpy should copy exactly n bytes");
}

#[test]
fn test_stpncpy_zero_n() {
    let src: Vec<c_char> = make_c_string(b"abc");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 1];
    let ret: *mut c_char = unsafe { stpncpy(dest.as_mut_ptr(), src.as_ptr(), 0) };
    let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
    assert_eq!(offset, 0, "stpncpy with n == 0 should return the destination start");
    assert_eq!(dest[0], 0x7F as c_char, "stpncpy with n == 0 must not write anything");
}
