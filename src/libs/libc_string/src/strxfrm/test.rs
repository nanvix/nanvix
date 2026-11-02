// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strxfrm;
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
fn test_strxfrm_identity_copy() {
    let src: Vec<c_char> = make_c_string(b"hello");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 8];
    let ret: c_size_t = unsafe { strxfrm(dest.as_mut_ptr(), src.as_ptr(), 8) };
    assert_eq!(ret as usize, 5, "strxfrm should return the untransformed source length");
    assert_eq!(
        &dest[..src.len()],
        &src[..],
        "strxfrm must copy the string verbatim in the C/POSIX locale"
    );
    assert_eq!(dest[6], 0x7F as c_char, "strxfrm must not write past the terminator");
    assert_eq!(dest[7], 0x7F as c_char, "strxfrm must not write past the terminator");
}

#[test]
fn test_strxfrm_truncates_to_capacity() {
    let src: Vec<c_char> = make_c_string(b"hello");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 8];
    // Capacity 3 leaves room for two bytes plus the terminator.
    let ret: c_size_t = unsafe { strxfrm(dest.as_mut_ptr(), src.as_ptr(), 3) };
    assert_eq!(ret as usize, 5, "strxfrm must return the untruncated source length");
    let expected: Vec<c_char> = make_c_string(b"he");
    assert_eq!(
        &dest[..3],
        &expected[..],
        "strxfrm must truncate and null-terminate within the capacity"
    );
    assert_eq!(dest[3], 0x7F as c_char, "strxfrm must not write past the capacity");
}

#[test]
fn test_strxfrm_exact_fit() {
    let src: Vec<c_char> = make_c_string(b"abc");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 8];
    // Capacity equals the source length plus the terminator.
    let ret: c_size_t = unsafe { strxfrm(dest.as_mut_ptr(), src.as_ptr(), 4) };
    assert_eq!(ret as usize, 3, "strxfrm should return the source length");
    assert_eq!(
        &dest[..4],
        &src[..],
        "strxfrm should copy the whole string including the terminator"
    );
    assert_eq!(dest[4], 0x7F as c_char, "strxfrm must not write past the terminator");
}

#[test]
fn test_strxfrm_zero_capacity_writes_nothing() {
    let src: Vec<c_char> = make_c_string(b"data");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 4];
    let ret: c_size_t = unsafe { strxfrm(dest.as_mut_ptr(), src.as_ptr(), 0) };
    assert_eq!(ret as usize, 4, "strxfrm should report the source length even with zero capacity");
    assert!(
        dest.iter().all(|&b| b == 0x7F as c_char),
        "strxfrm must not write anything when the capacity is zero"
    );
}

#[test]
fn test_strxfrm_zero_capacity_allows_null_destination() {
    let src: Vec<c_char> = make_c_string(b"data");
    let ret: c_size_t = unsafe { strxfrm(::core::ptr::null_mut(), src.as_ptr(), 0) };
    assert_eq!(
        ret as usize, 4,
        "strxfrm should support zero-capacity length queries with a null destination"
    );
}

#[test]
fn test_strxfrm_empty_source() {
    let src: Vec<c_char> = make_c_string(b"");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 4];
    let ret: c_size_t = unsafe { strxfrm(dest.as_mut_ptr(), src.as_ptr(), 4) };
    assert_eq!(ret as usize, 0, "strxfrm of an empty string should return zero");
    assert_eq!(dest[0], 0 as c_char, "strxfrm of an empty string should write just the terminator");
    assert_eq!(dest[1], 0x7F as c_char, "strxfrm must not write past the terminator");
}
