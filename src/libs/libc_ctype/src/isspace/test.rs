// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::isspace;
use ::sysapi::ffi::c_int;

#[test]
fn test_isspace_space() {
    assert_ne!(isspace(c_int::from(b' ')), 0);
}

#[test]
fn test_isspace_tab() {
    assert_ne!(isspace(0x09), 0); // '\t'
}

#[test]
fn test_isspace_newline() {
    assert_ne!(isspace(0x0A), 0); // '\n'
}

#[test]
fn test_isspace_vertical_tab() {
    assert_ne!(isspace(0x0B), 0); // '\v'
}

#[test]
fn test_isspace_form_feed() {
    assert_ne!(isspace(0x0C), 0); // '\f'
}

#[test]
fn test_isspace_carriage_return() {
    assert_ne!(isspace(0x0D), 0); // '\r'
}

#[test]
fn test_isspace_non_whitespace() {
    assert_eq!(isspace(c_int::from(b'A')), 0);
    assert_eq!(isspace(c_int::from(b'0')), 0);
    assert_eq!(isspace(c_int::from(b'!')), 0);
}

#[test]
fn test_isspace_eof() {
    assert_eq!(isspace(-1), 0);
}
