// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    lconv,
    localeconv,
};
use ::sysapi::ffi::c_char;

#[test]
fn test_localeconv_returns_non_null() {
    let ret: *mut lconv = localeconv();
    assert!(!ret.is_null());
}

#[test]
fn test_localeconv_decimal_point() {
    let lc: &lconv = unsafe { &*localeconv() };
    assert!(!lc.decimal_point.is_null());
    assert_eq!(unsafe { *lc.decimal_point }, c_char::try_from(b'.').expect("should fit in c_char"));
}

#[test]
fn test_localeconv_thousands_sep_empty() {
    let lc: &lconv = unsafe { &*localeconv() };
    assert!(!lc.thousands_sep.is_null());
    assert_eq!(unsafe { *lc.thousands_sep }, 0);
}

#[test]
fn test_localeconv_frac_digits() {
    let lc: &lconv = unsafe { &*localeconv() };
    assert_eq!(lc.frac_digits, i8::MAX);
}
