// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wctob_ascii() {
    assert_eq!(wctob(0x41), 0x41); // 'A'
    assert_eq!(wctob(0), 0); // null
    assert_eq!(wctob(127), 127); // DEL
}

#[test]
fn test_wctob_posix_locale_accepts_all_bytes() {
    assert_eq!(wctob(128), 128);
    assert_eq!(wctob(255), 255);
}

#[test]
fn test_wctob_out_of_byte_range() {
    assert_eq!(wctob(256), EOF);
    assert_eq!(wctob(0x1F600), EOF); // emoji codepoint
}

#[test]
fn test_wctob_weof() {
    assert_eq!(wctob(-1), EOF);
}
