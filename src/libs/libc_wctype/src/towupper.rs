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
/// Converts a wide character to uppercase.
///
/// # Parameters
///
/// - `wc`: Wide character to convert.
///
/// # Return Value
///
/// The uppercase equivalent if the character is a lowercase letter, otherwise the character
/// unchanged.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn towupper(wc: wint_t) -> wint_t {
    if (0x61..=0x7A).contains(&wc) {
        wc - 0x20
    } else {
        wc
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::towupper;
    use crate::wint_t::wint_t;

    #[test]
    fn test_towupper_lowercase() {
        for c in 0x61_i32..=0x7A_i32 {
            assert_eq!(towupper(c as wint_t), c - 0x20);
        }
    }

    #[test]
    fn test_towupper_uppercase() {
        for c in 0x41_i32..=0x5A_i32 {
            assert_eq!(towupper(c as wint_t), c);
        }
    }

    #[test]
    fn test_towupper_non_alpha() {
        assert_eq!(towupper(0x30), 0x30); // '0'
        assert_eq!(towupper(0x20), 0x20); // space
        assert_eq!(towupper(-1), -1); // WEOF
    }
}
