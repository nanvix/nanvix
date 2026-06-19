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
/// Tests whether a wide character is alphabetic.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is alphabetic, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswalpha(wc: wint_t) -> c_int {
    if (0x41..=0x5A).contains(&wc) || (0x61..=0x7A).contains(&wc) {
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
    use super::iswalpha;
    use crate::wint_t::wint_t;

    #[test]
    fn test_iswalpha_uppercase() {
        for c in 0x41_i32..=0x5A_i32 {
            assert_ne!(iswalpha(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswalpha_lowercase() {
        for c in 0x61_i32..=0x7A_i32 {
            assert_ne!(iswalpha(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswalpha_digit() {
        for c in 0x30_i32..=0x39_i32 {
            assert_eq!(iswalpha(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswalpha_special() {
        assert_eq!(iswalpha(0x20), 0); // space
        assert_eq!(iswalpha(-1), 0); // WEOF
    }
}
