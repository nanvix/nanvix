// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswlower;
use crate::wint_t::wint_t;

#[test]
fn test_iswlower_lowercase() {
    for c in 0x61_i32..=0x7A_i32 {
        assert_ne!(iswlower(c as wint_t), 0);
    }
}

#[test]
fn test_iswlower_uppercase() {
    for c in 0x41_i32..=0x5A_i32 {
        assert_eq!(iswlower(c as wint_t), 0);
    }
}

#[test]
fn test_iswlower_special() {
    assert_eq!(iswlower(0x30), 0); // '0'
    assert_eq!(iswlower(-1), 0); // WEOF
}
