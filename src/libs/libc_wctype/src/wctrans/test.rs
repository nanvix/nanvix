// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    wctrans,
    WCTRANS_TOLOWER,
    WCTRANS_TOUPPER,
};

#[test]
fn test_wctrans_valid_mappings() {
    let tolower = b"tolower\0";
    let toupper = b"toupper\0";
    assert_eq!(unsafe { wctrans(tolower.as_ptr().cast()) }, WCTRANS_TOLOWER);
    assert_eq!(unsafe { wctrans(toupper.as_ptr().cast()) }, WCTRANS_TOUPPER);
}

#[test]
fn test_wctrans_invalid_mapping() {
    let invalid = b"titlecase\0";
    assert_eq!(unsafe { wctrans(invalid.as_ptr().cast()) }, 0);
}

#[test]
fn test_wctrans_null_mapping() {
    assert_eq!(unsafe { wctrans(::core::ptr::null()) }, 0);
}
