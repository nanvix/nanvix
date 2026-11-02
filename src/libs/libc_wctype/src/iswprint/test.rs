// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswprint;
use crate::wint_t::wint_t;

#[test]
fn test_iswprint_printable() {
    for c in 0x20_i32..=0x7E_i32 {
        assert_ne!(iswprint(c as wint_t), 0);
    }
}

#[test]
fn test_iswprint_control() {
    for c in 0x00_i32..=0x1F_i32 {
        assert_eq!(iswprint(c as wint_t), 0);
    }
}

#[test]
fn test_iswprint_special() {
    assert_eq!(iswprint(0x7F), 0); // DEL
    assert_eq!(iswprint(-1), 0); // WEOF
}
