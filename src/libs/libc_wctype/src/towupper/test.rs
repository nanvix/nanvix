// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::towupper;
use crate::wint_t::wint_t;

#[test]
fn test_towupper_lowercase() {
    for c in 0x61_i32..=0x7A_i32 {
        assert_eq!(towupper(c as wint_t), c - 0x20);
    }
}

#[test]
fn test_towupper_uppercase() {
    for c in 0x41_i32..=0x5A_i32 {
        assert_eq!(towupper(c as wint_t), c);
    }
}

#[test]
fn test_towupper_non_alpha() {
    assert_eq!(towupper(0x30), 0x30); // '0'
    assert_eq!(towupper(0x20), 0x20); // space
    assert_eq!(towupper(-1), -1); // WEOF
}
