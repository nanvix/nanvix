// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isupper;
use ::sysapi::ffi::c_int;

#[test]
fn test_isupper_uppercase() {
    for c in b'A'..=b'Z' {
        assert_ne!(
            isupper(c_int::from(c)),
            0,
            "isupper should return non-zero for '{}'",
            c as char
        );
    }
}

#[test]
fn test_isupper_lowercase() {
    for c in b'a'..=b'z' {
        assert_eq!(isupper(c_int::from(c)), 0);
    }
}

#[test]
fn test_isupper_digits() {
    for c in b'0'..=b'9' {
        assert_eq!(isupper(c_int::from(c)), 0);
    }
}

#[test]
fn test_isupper_eof() {
    assert_eq!(isupper(-1), 0);
}
