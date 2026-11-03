// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isalnum;
use ::sysapi::ffi::c_int;

#[test]
fn test_isalnum_uppercase() {
    for c in b'A'..=b'Z' {
        assert_ne!(
            isalnum(c_int::from(c)),
            0,
            "isalnum should return non-zero for '{}'",
            c as char
        );
    }
}

#[test]
fn test_isalnum_lowercase() {
    for c in b'a'..=b'z' {
        assert_ne!(isalnum(c_int::from(c)), 0);
    }
}

#[test]
fn test_isalnum_digits() {
    for c in b'0'..=b'9' {
        assert_ne!(isalnum(c_int::from(c)), 0);
    }
}

#[test]
fn test_isalnum_special() {
    assert_eq!(isalnum(c_int::from(b' ')), 0);
    assert_eq!(isalnum(c_int::from(b'!')), 0);
    assert_eq!(isalnum(c_int::from(b'@')), 0);
    assert_eq!(isalnum(-1), 0); // EOF
}
