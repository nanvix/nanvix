// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswpunct;

#[test]
fn test_iswpunct_punctuation() {
    assert_ne!(iswpunct(0x21), 0); // '!'
    assert_ne!(iswpunct(0x2F), 0); // '/'
    assert_ne!(iswpunct(0x3A), 0); // ':'
    assert_ne!(iswpunct(0x40), 0); // '@'
    assert_ne!(iswpunct(0x5B), 0); // '['
    assert_ne!(iswpunct(0x60), 0); // '`'
    assert_ne!(iswpunct(0x7B), 0); // '{'
    assert_ne!(iswpunct(0x7E), 0); // '~'
}

#[test]
fn test_iswpunct_non_punct() {
    assert_eq!(iswpunct(0x41), 0); // 'A'
    assert_eq!(iswpunct(0x30), 0); // '0'
    assert_eq!(iswpunct(0x20), 0); // space
}

#[test]
fn test_iswpunct_special() {
    assert_eq!(iswpunct(0x00), 0); // null
    assert_eq!(iswpunct(-1), 0); // WEOF
}
