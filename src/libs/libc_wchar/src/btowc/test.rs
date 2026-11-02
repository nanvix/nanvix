// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_btowc_ascii() {
    assert_eq!(btowc(0x41), 0x41); // 'A'
    assert_eq!(btowc(0), 0); // null
    assert_eq!(btowc(127), 127); // DEL
}

#[test]
fn test_btowc_posix_locale_accepts_all_bytes() {
    assert_eq!(btowc(128), 128);
    assert_eq!(btowc(255), 255);
}

#[test]
fn test_btowc_eof() {
    assert_eq!(btowc(-1), WEOF);
}

#[test]
fn test_btowc_out_of_range() {
    assert_eq!(btowc(-2), WEOF);
    assert_eq!(btowc(256), WEOF);
}
