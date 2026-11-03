// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isxdigit;
use ::sysapi::ffi::c_int;

#[test]
fn test_isxdigit_digits() {
    for c in b'0'..=b'9' {
        assert_ne!(isxdigit(c_int::from(c)), 0);
    }
}

#[test]
fn test_isxdigit_upper_hex() {
    for c in b'A'..=b'F' {
        assert_ne!(isxdigit(c_int::from(c)), 0);
    }
}

#[test]
fn test_isxdigit_lower_hex() {
    for c in b'a'..=b'f' {
        assert_ne!(isxdigit(c_int::from(c)), 0);
    }
}

#[test]
fn test_isxdigit_non_hex() {
    assert_eq!(isxdigit(c_int::from(b'G')), 0);
    assert_eq!(isxdigit(c_int::from(b'g')), 0);
    assert_eq!(isxdigit(c_int::from(b'Z')), 0);
    assert_eq!(isxdigit(c_int::from(b'!')), 0);
}

#[test]
fn test_isxdigit_eof() {
    assert_eq!(isxdigit(-1), 0);
}
