// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswupper;
use crate::wint_t::wint_t;

#[test]
fn test_iswupper_uppercase() {
    for c in 0x41_i32..=0x5A_i32 {
        assert_ne!(iswupper(c as wint_t), 0);
    }
}

#[test]
fn test_iswupper_lowercase() {
    for c in 0x61_i32..=0x7A_i32 {
        assert_eq!(iswupper(c as wint_t), 0);
    }
}

#[test]
fn test_iswupper_special() {
    assert_eq!(iswupper(0x30), 0); // '0'
    assert_eq!(iswupper(-1), 0); // WEOF
}
