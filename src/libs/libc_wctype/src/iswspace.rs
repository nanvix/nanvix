// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::wint_t::wint_t;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests whether a wide character is a whitespace character.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is whitespace, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswspace(wc: wint_t) -> c_int {
    if wc == 0x20 || (0x09..=0x0D).contains(&wc) {
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
    use super::iswspace;

    #[test]
    fn test_iswspace_whitespace() {
        assert_ne!(iswspace(0x20), 0); // space
        assert_ne!(iswspace(0x09), 0); // tab
        assert_ne!(iswspace(0x0A), 0); // newline
        assert_ne!(iswspace(0x0B), 0); // vertical tab
        assert_ne!(iswspace(0x0C), 0); // form feed
        assert_ne!(iswspace(0x0D), 0); // carriage return
    }

    #[test]
    fn test_iswspace_non_whitespace() {
        assert_eq!(iswspace(0x41), 0); // 'A'
        assert_eq!(iswspace(0x30), 0); // '0'
    }

    #[test]
    fn test_iswspace_special() {
        assert_eq!(iswspace(0x00), 0); // null
        assert_eq!(iswspace(-1), 0); // WEOF
    }
}
