// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswalpha;
use crate::wint_t::wint_t;

#[test]
fn test_iswalpha_uppercase() {
    for c in 0x41_i32..=0x5A_i32 {
        assert_ne!(iswalpha(c as wint_t), 0);
    }
}

#[test]
fn test_iswalpha_lowercase() {
    for c in 0x61_i32..=0x7A_i32 {
        assert_ne!(iswalpha(c as wint_t), 0);
    }
}

#[test]
fn test_iswalpha_digit() {
    for c in 0x30_i32..=0x39_i32 {
        assert_eq!(iswalpha(c as wint_t), 0);
    }
}

#[test]
fn test_iswalpha_special() {
    assert_eq!(iswalpha(0x20), 0); // space
    assert_eq!(iswalpha(-1), 0); // WEOF
}
