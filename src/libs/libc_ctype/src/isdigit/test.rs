// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isdigit;
use ::sysapi::ffi::c_int;

#[test]
fn test_isdigit_digits() {
    for c in b'0'..=b'9' {
        assert_ne!(
            isdigit(c_int::from(c)),
            0,
            "isdigit should return non-zero for '{}'",
            c as char
        );
    }
}

#[test]
fn test_isdigit_letters() {
    for c in b'A'..=b'Z' {
        assert_eq!(isdigit(c_int::from(c)), 0);
    }
    for c in b'a'..=b'z' {
        assert_eq!(isdigit(c_int::from(c)), 0);
    }
}

#[test]
fn test_isdigit_special() {
    assert_eq!(isdigit(c_int::from(b' ')), 0);
    assert_eq!(isdigit(c_int::from(b'!')), 0);
    assert_eq!(isdigit(c_int::from(b'/')), 0);
    assert_eq!(isdigit(c_int::from(b':')), 0);
    assert_eq!(isdigit(-1), 0); // EOF
}
