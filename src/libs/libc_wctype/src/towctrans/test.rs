// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::towctrans;
use crate::wctrans::{
    WCTRANS_TOLOWER,
    WCTRANS_TOUPPER,
};

#[test]
fn test_towctrans_valid_mappings() {
    assert_eq!(towctrans(0x41, WCTRANS_TOLOWER), 0x61); // 'A' -> 'a'
    assert_eq!(towctrans(0x61, WCTRANS_TOUPPER), 0x41); // 'a' -> 'A'
}

#[test]
fn test_towctrans_zero_descriptor() {
    assert_eq!(towctrans(0x41, 0), 0x41); // 'A'
}

#[test]
fn test_towctrans_invalid_descriptor() {
    assert_eq!(towctrans(0x41, 999), 0x41); // 'A'
}
