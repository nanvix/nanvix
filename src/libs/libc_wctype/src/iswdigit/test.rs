// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswdigit;
use crate::wint_t::wint_t;

#[test]
fn test_iswdigit_digits() {
    for c in 0x30_i32..=0x39_i32 {
        assert_ne!(iswdigit(c as wint_t), 0);
    }
}

#[test]
fn test_iswdigit_alpha() {
    for c in 0x41_i32..=0x5A_i32 {
        assert_eq!(iswdigit(c as wint_t), 0);
    }
}

#[test]
fn test_iswdigit_special() {
    assert_eq!(iswdigit(0x20), 0); // space
    assert_eq!(iswdigit(-1), 0); // WEOF
}
