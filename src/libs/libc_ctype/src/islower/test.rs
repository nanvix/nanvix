// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::islower;
use ::sysapi::ffi::c_int;

#[test]
fn test_islower_lowercase() {
    for c in b'a'..=b'z' {
        assert_ne!(
            islower(c_int::from(c)),
            0,
            "islower should return non-zero for '{}'",
            c as char
        );
    }
}

#[test]
fn test_islower_uppercase() {
    for c in b'A'..=b'Z' {
        assert_eq!(islower(c_int::from(c)), 0);
    }
}

#[test]
fn test_islower_digits() {
    for c in b'0'..=b'9' {
        assert_eq!(islower(c_int::from(c)), 0);
    }
}

#[test]
fn test_islower_eof() {
    assert_eq!(islower(-1), 0);
}
