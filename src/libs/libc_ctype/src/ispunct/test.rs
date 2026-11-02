// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ispunct;
use ::sysapi::ffi::c_int;

#[test]
fn test_ispunct_punctuation() {
    let puncts = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
    for &c in puncts {
        assert_ne!(
            ispunct(c_int::from(c)),
            0,
            "ispunct should return non-zero for '{}'",
            c as char
        );
    }
}

#[test]
fn test_ispunct_letters() {
    for c in b'A'..=b'Z' {
        assert_eq!(ispunct(c_int::from(c)), 0);
    }
    for c in b'a'..=b'z' {
        assert_eq!(ispunct(c_int::from(c)), 0);
    }
}

#[test]
fn test_ispunct_digits() {
    for c in b'0'..=b'9' {
        assert_eq!(ispunct(c_int::from(c)), 0);
    }
}

#[test]
fn test_ispunct_space() {
    assert_eq!(ispunct(c_int::from(b' ')), 0);
}

#[test]
fn test_ispunct_eof() {
    assert_eq!(ispunct(-1), 0);
}
