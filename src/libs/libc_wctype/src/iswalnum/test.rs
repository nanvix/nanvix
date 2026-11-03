// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswalnum;
use crate::wint_t::wint_t;

#[test]
fn test_iswalnum_alpha() {
    assert_ne!(iswalnum(0x41), 0); // 'A'
    assert_ne!(iswalnum(0x7A), 0); // 'z'
}

#[test]
fn test_iswalnum_digit() {
    for c in 0x30_i32..=0x39_i32 {
        assert_ne!(iswalnum(c as wint_t), 0);
    }
}

#[test]
fn test_iswalnum_non_alnum() {
    assert_eq!(iswalnum(0x20), 0); // space
    assert_eq!(iswalnum(0x21), 0); // '!'
    assert_eq!(iswalnum(-1), 0); // WEOF
}
