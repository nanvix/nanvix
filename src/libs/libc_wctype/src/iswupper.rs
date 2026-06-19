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
/// Tests whether a wide character is an uppercase letter.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is uppercase, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswupper(wc: wint_t) -> c_int {
    if (0x41..=0x5A).contains(&wc) {
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
    use super::iswupper;
    use crate::wint_t::wint_t;

    #[test]
    fn test_iswupper_uppercase() {
        for c in 0x41_i32..=0x5A_i32 {
            assert_ne!(iswupper(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswupper_lowercase() {
        for c in 0x61_i32..=0x7A_i32 {
            assert_eq!(iswupper(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswupper_special() {
        assert_eq!(iswupper(0x30), 0); // '0'
        assert_eq!(iswupper(-1), 0); // WEOF
    }
}
