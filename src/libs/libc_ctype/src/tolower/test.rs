// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::tolower;
use ::sysapi::ffi::c_int;

#[test]
fn test_tolower_uppercase() {
    for c in b'A'..=b'Z' {
        let expected = c + 32;
        assert_eq!(
            tolower(c_int::from(c)),
            c_int::from(expected),
            "tolower('{}') should return '{}'",
            c as char,
            expected as char
        );
    }
}

#[test]
fn test_tolower_already_lower() {
    for c in b'a'..=b'z' {
        assert_eq!(tolower(c_int::from(c)), c_int::from(c));
    }
}

#[test]
fn test_tolower_digit() {
    for c in b'0'..=b'9' {
        assert_eq!(tolower(c_int::from(c)), c_int::from(c));
    }
}

#[test]
fn test_tolower_eof() {
    assert_eq!(tolower(-1), -1);
}
