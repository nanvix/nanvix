// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isgraph;
use ::sysapi::ffi::c_int;

#[test]
fn test_isgraph_graphic() {
    for c in 0x21..=0x7E {
        assert_ne!(isgraph(c), 0, "isgraph should return non-zero for 0x{:02X}", c);
    }
}

#[test]
fn test_isgraph_space() {
    assert_eq!(isgraph(c_int::from(b' ')), 0);
}

#[test]
fn test_isgraph_control() {
    for c in 0x00..0x20 {
        assert_eq!(isgraph(c), 0, "isgraph should return zero for 0x{:02X}", c);
    }
}

#[test]
fn test_isgraph_del() {
    assert_eq!(isgraph(0x7F), 0);
}

#[test]
fn test_isgraph_eof() {
    assert_eq!(isgraph(-1), 0);
}
