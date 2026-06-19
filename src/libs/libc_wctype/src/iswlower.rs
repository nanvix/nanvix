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
/// Tests whether a wide character is a lowercase letter.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is lowercase, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswlower(wc: wint_t) -> c_int {
    if (0x61..=0x7A).contains(&wc) {
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
    use super::iswlower;
    use crate::wint_t::wint_t;

    #[test]
    fn test_iswlower_lowercase() {
        for c in 0x61_i32..=0x7A_i32 {
            assert_ne!(iswlower(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswlower_uppercase() {
        for c in 0x41_i32..=0x5A_i32 {
            assert_eq!(iswlower(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswlower_special() {
        assert_eq!(iswlower(0x30), 0); // '0'
        assert_eq!(iswlower(-1), 0); // WEOF
    }
}
