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
/// Tests whether a wide character is printable.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is printable, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswprint(wc: wint_t) -> c_int {
    if (0x20..=0x7E).contains(&wc) {
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
    use super::iswprint;
    use crate::wint_t::wint_t;

    #[test]
    fn test_iswprint_printable() {
        for c in 0x20_i32..=0x7E_i32 {
            assert_ne!(iswprint(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswprint_control() {
        for c in 0x00_i32..=0x1F_i32 {
            assert_eq!(iswprint(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswprint_special() {
        assert_eq!(iswprint(0x7F), 0); // DEL
        assert_eq!(iswprint(-1), 0); // WEOF
    }
}
