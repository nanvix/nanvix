// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    randomize_suffix,
    validate,
    SUFFIX_LEN,
};
use ::sysapi::ffi::c_char;

#[test]
fn test_validate_accepts_suffix() {
    let tmpl: &[u8] = b"foo.XXXXXX\0";
    let len: Option<usize> = unsafe { validate(tmpl.as_ptr().cast::<c_char>()) };
    assert_eq!(len, Some(10));
}

#[test]
fn test_validate_rejects_short() {
    let tmpl: &[u8] = b"XXXXX\0";
    assert_eq!(unsafe { validate(tmpl.as_ptr().cast::<c_char>()) }, None);
}

#[test]
fn test_validate_rejects_missing_suffix() {
    let tmpl: &[u8] = b"foobar\0";
    assert_eq!(unsafe { validate(tmpl.as_ptr().cast::<c_char>()) }, None);
}

#[test]
fn test_validate_rejects_null() {
    assert_eq!(unsafe { validate(core::ptr::null()) }, None);
}

#[test]
fn test_randomize_replaces_suffix() {
    let mut tmpl: [u8; 11] = *b"foo.XXXXXX\0";
    let len: usize = unsafe { validate(tmpl.as_ptr().cast::<c_char>()) }.unwrap_or(0);
    assert_eq!(len, 10);
    unsafe { randomize_suffix(tmpl.as_mut_ptr().cast::<c_char>(), len) };

    // The fixed prefix and null terminator are untouched.
    assert_eq!(&tmpl[..4], b"foo.");
    assert_eq!(tmpl[10], 0);
    // Every suffix byte is replaced with an alphanumeric character from the alphabet.
    for &b in &tmpl[len - SUFFIX_LEN..len] {
        assert!(b.is_ascii_alphanumeric(), "byte {b:#x} not in alphabet");
    }
}
