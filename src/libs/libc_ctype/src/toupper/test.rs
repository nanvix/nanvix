// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::toupper;
use ::sysapi::ffi::c_int;

#[test]
fn test_toupper_lowercase() {
    for c in b'a'..=b'z' {
        let expected = c - 32;
        assert_eq!(
            toupper(c_int::from(c)),
            c_int::from(expected),
            "toupper('{}') should return '{}'",
            c as char,
            expected as char
        );
    }
}

#[test]
fn test_toupper_already_upper() {
    for c in b'A'..=b'Z' {
        assert_eq!(toupper(c_int::from(c)), c_int::from(c));
    }
}

#[test]
fn test_toupper_digit() {
    for c in b'0'..=b'9' {
        assert_eq!(toupper(c_int::from(c)), c_int::from(c));
    }
}

#[test]
fn test_toupper_eof() {
    assert_eq!(toupper(-1), -1);
}
