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
    iswalpha::iswalpha,
    iswdigit::iswdigit,
    wint_t::wint_t,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests whether a wide character is alphanumeric.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is alphanumeric, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswalnum(wc: wint_t) -> c_int {
    if iswalpha(wc) != 0 || iswdigit(wc) != 0 {
        1
    } else {
        0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::iswalnum;
    use crate::wint_t::wint_t;

    #[test]
    fn test_iswalnum_alpha() {
        assert_ne!(iswalnum(0x41), 0); // 'A'
        assert_ne!(iswalnum(0x7A), 0); // 'z'
    }

    #[test]
    fn test_iswalnum_digit() {
        for c in 0x30_i32..=0x39_i32 {
            assert_ne!(iswalnum(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswalnum_non_alnum() {
        assert_eq!(iswalnum(0x20), 0); // space
        assert_eq!(iswalnum(0x21), 0); // '!'
        assert_eq!(iswalnum(-1), 0); // WEOF
    }
}
