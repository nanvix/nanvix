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
    iswdigit::iswdigit,
    iswgraph::iswgraph,
    iswlower::iswlower,
    iswprint::iswprint,
    iswpunct::iswpunct,
    iswspace::iswspace,
    iswupper::iswupper,
    iswxdigit::iswxdigit,
    wctype::{
        WCTYPE_ALNUM,
        WCTYPE_ALPHA,
        WCTYPE_BLANK,
        WCTYPE_CNTRL,
        WCTYPE_DIGIT,
        WCTYPE_GRAPH,
        WCTYPE_LOWER,
        WCTYPE_PRINT,
        WCTYPE_PUNCT,
        WCTYPE_SPACE,
        WCTYPE_UPPER,
        WCTYPE_XDIGIT,
    },
    wctype_t::wctype_t,
    wint_t::wint_t,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests whether a wide character belongs to a specified character class.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
/// - `charclass`: Character class descriptor returned by `wctype()`.
///
/// # Return Value
///
/// Non-zero if the character belongs to the class, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswctype(wc: wint_t, charclass: wctype_t) -> c_int {
    match charclass {
        WCTYPE_ALNUM => iswalnum(wc),
        WCTYPE_ALPHA => iswalpha(wc),
        WCTYPE_BLANK => iswblank(wc),
        WCTYPE_CNTRL => iswcntrl(wc),
        WCTYPE_DIGIT => iswdigit(wc),
        WCTYPE_GRAPH => iswgraph(wc),
        WCTYPE_LOWER => iswlower(wc),
        WCTYPE_PRINT => iswprint(wc),
        WCTYPE_PUNCT => iswpunct(wc),
        WCTYPE_SPACE => iswspace(wc),
        WCTYPE_UPPER => iswupper(wc),
        WCTYPE_XDIGIT => iswxdigit(wc),
        _ => 0,
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
