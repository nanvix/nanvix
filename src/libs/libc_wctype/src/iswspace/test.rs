// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswspace;

#[test]
fn test_iswspace_whitespace() {
    assert_ne!(iswspace(0x20), 0); // space
    assert_ne!(iswspace(0x09), 0); // tab
    assert_ne!(iswspace(0x0A), 0); // newline
    assert_ne!(iswspace(0x0B), 0); // vertical tab
    assert_ne!(iswspace(0x0C), 0); // form feed
    assert_ne!(iswspace(0x0D), 0); // carriage return
}

#[test]
fn test_iswspace_non_whitespace() {
    assert_eq!(iswspace(0x41), 0); // 'A'
    assert_eq!(iswspace(0x30), 0); // '0'
}

#[test]
fn test_iswspace_special() {
    assert_eq!(iswspace(0x00), 0); // null
    assert_eq!(iswspace(-1), 0); // WEOF
}
