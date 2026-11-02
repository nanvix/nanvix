// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswgraph;
use crate::wint_t::wint_t;

#[test]
fn test_iswgraph_graphic() {
    for c in 0x21_i32..=0x7E_i32 {
        assert_ne!(iswgraph(c as wint_t), 0);
    }
}

#[test]
fn test_iswgraph_space() {
    assert_eq!(iswgraph(0x20), 0); // space is not graphic
}

#[test]
fn test_iswgraph_control() {
    for c in 0x00_i32..=0x1F_i32 {
        assert_eq!(iswgraph(c as wint_t), 0);
    }
    assert_eq!(iswgraph(0x7F), 0); // DEL
}

#[test]
fn test_iswgraph_special() {
    assert_eq!(iswgraph(-1), 0); // WEOF
}
