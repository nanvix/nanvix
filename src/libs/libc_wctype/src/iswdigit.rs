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
/// Tests whether a wide character is a decimal digit.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is a digit, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswdigit(wc: wint_t) -> c_int {
    if (0x30..=0x39).contains(&wc) {
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
    use super::iswdigit;
    use crate::wint_t::wint_t;

    #[test]
    fn test_iswdigit_digits() {
        for c in 0x30_i32..=0x39_i32 {
            assert_ne!(iswdigit(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswdigit_alpha() {
        for c in 0x41_i32..=0x5A_i32 {
            assert_eq!(iswdigit(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswdigit_special() {
        assert_eq!(iswdigit(0x20), 0); // space
        assert_eq!(iswdigit(-1), 0); // WEOF
    }
}
