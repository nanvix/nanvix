// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isalpha;
use ::sysapi::ffi::c_int;

#[test]
fn test_isalpha_uppercase() {
    for c in b'A'..=b'Z' {
        assert_ne!(
            isalpha(c_int::from(c)),
            0,
            "isalpha should return non-zero for '{}'",
            c as char
        );
    }
}

#[test]
fn test_isalpha_lowercase() {
    for c in b'a'..=b'z' {
        assert_ne!(isalpha(c_int::from(c)), 0);
    }
}

#[test]
fn test_isalpha_digit() {
    for c in b'0'..=b'9' {
        assert_eq!(isalpha(c_int::from(c)), 0);
    }
}

#[test]
fn test_isalpha_special() {
    assert_eq!(isalpha(c_int::from(b' ')), 0);
    assert_eq!(isalpha(c_int::from(b'!')), 0);
    assert_eq!(isalpha(-1), 0); // EOF
}
