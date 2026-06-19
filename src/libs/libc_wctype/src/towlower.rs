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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a wide character to lowercase.
///
/// # Parameters
///
/// - `wc`: Wide character to convert.
///
/// # Return Value
///
/// The lowercase equivalent if the character is an uppercase letter, otherwise the character
/// unchanged.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn towlower(wc: wint_t) -> wint_t {
    if (0x41..=0x5A).contains(&wc) {
        wc + 0x20
    } else {
        wc
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::towlower;
    use crate::wint_t::wint_t;

    #[test]
    fn test_towlower_uppercase() {
        for c in 0x41_i32..=0x5A_i32 {
            assert_eq!(towlower(c as wint_t), c + 0x20);
        }
    }

    #[test]
    fn test_towlower_lowercase() {
        for c in 0x61_i32..=0x7A_i32 {
            assert_eq!(towlower(c as wint_t), c);
        }
    }

    #[test]
    fn test_towlower_non_alpha() {
        assert_eq!(towlower(0x30), 0x30); // '0'
        assert_eq!(towlower(0x20), 0x20); // space
        assert_eq!(towlower(-1), -1); // WEOF
    }
}
