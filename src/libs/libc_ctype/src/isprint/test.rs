// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isprint;

#[test]
fn test_isprint_printable() {
    for c in 0x20..=0x7E {
        assert_ne!(isprint(c), 0, "isprint should return non-zero for 0x{:02X}", c);
    }
}

#[test]
fn test_isprint_control_chars() {
    for c in 0x00..0x20 {
        assert_eq!(isprint(c), 0, "isprint should return zero for 0x{:02X}", c);
    }
}

#[test]
fn test_isprint_del() {
    assert_eq!(isprint(0x7F), 0);
}

#[test]
fn test_isprint_eof() {
    assert_eq!(isprint(-1), 0);
}
