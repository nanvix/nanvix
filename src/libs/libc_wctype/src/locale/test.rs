// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    iswalnum_l,
    iswctype_l,
    towctrans_l,
    wctrans_l,
    wctype_l,
};
use crate::{
    wctrans::WCTRANS_TOUPPER,
    wctype::WCTYPE_ALPHA,
};

#[test]
fn test_locale_wrappers_ignore_c_locale_handle() {
    let locale: *mut ::sysapi::ffi::c_void = ::core::ptr::null_mut();
    assert_ne!(iswalnum_l(0x41, locale), 0); // 'A'
    assert_ne!(iswctype_l(0x41, WCTYPE_ALPHA, locale), 0); // 'A'
    assert_eq!(towctrans_l(0x61, WCTRANS_TOUPPER, locale), 0x41); // 'a' -> 'A'
}

#[test]
fn test_locale_descriptor_wrappers() {
    let locale: *mut ::sysapi::ffi::c_void = ::core::ptr::null_mut();
    let alpha = b"alpha\0";
    let toupper = b"toupper\0";
    assert_eq!(unsafe { wctype_l(alpha.as_ptr().cast(), locale) }, WCTYPE_ALPHA);
    assert_eq!(unsafe { wctrans_l(toupper.as_ptr().cast(), locale) }, WCTRANS_TOUPPER);
}
