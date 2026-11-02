// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswxdigit;
use crate::wint_t::wint_t;

#[test]
fn test_iswxdigit_digits() {
    for c in 0x30_i32..=0x39_i32 {
        assert_ne!(iswxdigit(c as wint_t), 0);
    }
}

#[test]
fn test_iswxdigit_hex_upper() {
    for c in 0x41_i32..=0x46_i32 {
        assert_ne!(iswxdigit(c as wint_t), 0);
    }
    assert_eq!(iswxdigit(0x47), 0); // 'G'
}

#[test]
fn test_iswxdigit_hex_lower() {
    for c in 0x61_i32..=0x66_i32 {
        assert_ne!(iswxdigit(c as wint_t), 0);
    }
    assert_eq!(iswxdigit(0x67), 0); // 'g'
}

#[test]
fn test_iswxdigit_special() {
    assert_eq!(iswxdigit(0x20), 0); // space
    assert_eq!(iswxdigit(-1), 0); // WEOF
}
