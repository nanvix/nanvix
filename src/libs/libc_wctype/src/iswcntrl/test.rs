// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswcntrl;
use crate::wint_t::wint_t;

#[test]
fn test_iswcntrl_control() {
    for c in 0x00_i32..=0x1F_i32 {
        assert_ne!(iswcntrl(c as wint_t), 0);
    }
    assert_ne!(iswcntrl(0x7F), 0); // DEL
}

#[test]
fn test_iswcntrl_printable() {
    for c in 0x20_i32..=0x7E_i32 {
        assert_eq!(iswcntrl(c as wint_t), 0);
    }
}

#[test]
fn test_iswcntrl_special() {
    assert_eq!(iswcntrl(-1), 0); // WEOF
}
