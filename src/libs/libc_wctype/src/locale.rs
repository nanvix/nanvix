// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    iswalnum::iswalnum,
    iswalpha::iswalpha,
    iswblank::iswblank,
    iswcntrl::iswcntrl,
    iswctype::iswctype,
    iswdigit::iswdigit,
    iswgraph::iswgraph,
    iswlower::iswlower,
    iswprint::iswprint,
    iswpunct::iswpunct,
    iswspace::iswspace,
    iswupper::iswupper,
    iswxdigit::iswxdigit,
    towctrans::towctrans,
    towlower::towlower,
    towupper::towupper,
    wctrans::wctrans,
    wctrans_t::wctrans_t,
    wctype::wctype,
    wctype_t::wctype_t,
    wint_t::wint_t,
};
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_void,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether a wide character is alphanumeric in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswalnum_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswalnum(wc)
}

/// Tests whether a wide character is alphabetic in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswalpha_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswalpha(wc)
}

/// Tests whether a wide character is blank in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswblank_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswblank(wc)
}

/// Tests whether a wide character is a control character in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswcntrl_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswcntrl(wc)
}

/// Tests whether a wide character belongs to a class in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswctype_l(wc: wint_t, charclass: wctype_t, _locale: *mut c_void) -> c_int {
    iswctype(wc, charclass)
}

/// Tests whether a wide character is a decimal digit in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswdigit_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswdigit(wc)
}

/// Tests whether a wide character is graphic in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswgraph_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswgraph(wc)
}

/// Tests whether a wide character is lowercase in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswlower_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswlower(wc)
}

/// Tests whether a wide character is printable in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswprint_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswprint(wc)
}

/// Tests whether a wide character is punctuation in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswpunct_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswpunct(wc)
}

/// Tests whether a wide character is whitespace in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswspace_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswspace(wc)
}

/// Tests whether a wide character is uppercase in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswupper_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswupper(wc)
}

/// Tests whether a wide character is a hexadecimal digit in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswxdigit_l(wc: wint_t, _locale: *mut c_void) -> c_int {
    iswxdigit(wc)
}

/// Maps a wide character using a mapping descriptor in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn towctrans_l(wc: wint_t, desc: wctrans_t, _locale: *mut c_void) -> wint_t {
    towctrans(wc, desc)
}

/// Converts a wide character to lowercase in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn towlower_l(wc: wint_t, _locale: *mut c_void) -> wint_t {
    towlower(wc)
}

/// Converts a wide character to uppercase in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn towupper_l(wc: wint_t, _locale: *mut c_void) -> wint_t {
    towupper(wc)
}

/// Defines a wide-character mapping descriptor in the C/POSIX locale.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `charclass`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wctrans_l(charclass: *const c_char, _locale: *mut c_void) -> wctrans_t {
    unsafe { wctrans(charclass) }
}

/// Defines a wide-character classification descriptor in the C/POSIX locale.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `property`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wctype_l(property: *const c_char, _locale: *mut c_void) -> wctype_t {
    unsafe { wctype(property) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
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
}
