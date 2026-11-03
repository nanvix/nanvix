// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswblank;

#[test]
fn test_iswblank_blank() {
    assert_ne!(iswblank(0x20), 0); // space
    assert_ne!(iswblank(0x09), 0); // tab
}

#[test]
fn test_iswblank_other_whitespace() {
    assert_eq!(iswblank(0x0A), 0); // newline
    assert_eq!(iswblank(0x0D), 0); // carriage return
}

#[test]
fn test_iswblank_non_blank() {
    assert_eq!(iswblank(0x41), 0); // 'A'
    assert_eq!(iswblank(-1), 0); // WEOF
}
