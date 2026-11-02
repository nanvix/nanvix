// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isblank;

#[test]
fn test_isblank_space() {
    assert_ne!(isblank(0x20), 0);
}

#[test]
fn test_isblank_tab() {
    assert_ne!(isblank(0x09), 0);
}

#[test]
fn test_isblank_newline() {
    assert_eq!(isblank(0x0A), 0);
}

#[test]
fn test_isblank_other_whitespace() {
    assert_eq!(isblank(0x0B), 0); // '\v'
    assert_eq!(isblank(0x0C), 0); // '\f'
    assert_eq!(isblank(0x0D), 0); // '\r'
}

#[test]
fn test_isblank_eof() {
    assert_eq!(isblank(-1), 0);
}
