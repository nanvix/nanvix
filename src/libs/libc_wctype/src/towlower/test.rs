// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::towlower;
use crate::wint_t::wint_t;

#[test]
fn test_towlower_uppercase() {
    for c in 0x41_i32..=0x5A_i32 {
        assert_eq!(towlower(c as wint_t), c + 0x20);
    }
}

#[test]
fn test_towlower_lowercase() {
    for c in 0x61_i32..=0x7A_i32 {
        assert_eq!(towlower(c as wint_t), c);
    }
}

#[test]
fn test_towlower_non_alpha() {
    assert_eq!(towlower(0x30), 0x30); // '0'
    assert_eq!(towlower(0x20), 0x20); // space
    assert_eq!(towlower(-1), -1); // WEOF
}
