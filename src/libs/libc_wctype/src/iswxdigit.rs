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
/// Tests whether a wide character is a hexadecimal digit.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is a hexadecimal digit, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswxdigit(wc: wint_t) -> c_int {
    if (0x30..=0x39).contains(&wc) || (0x41..=0x46).contains(&wc) || (0x61..=0x66).contains(&wc) {
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
    use super::iswxdigit;
    use crate::wint_t::wint_t;

    #[test]
    fn test_iswxdigit_digits() {
        for c in 0x30_i32..=0x39_i32 {
            assert_ne!(iswxdigit(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswxdigit_hex_upper() {
        for c in 0x41_i32..=0x46_i32 {
            assert_ne!(iswxdigit(c as wint_t), 0);
        }
        assert_eq!(iswxdigit(0x47), 0); // 'G'
    }

    #[test]
    fn test_iswxdigit_hex_lower() {
        for c in 0x61_i32..=0x66_i32 {
            assert_ne!(iswxdigit(c as wint_t), 0);
        }
        assert_eq!(iswxdigit(0x67), 0); // 'g'
    }

    #[test]
    fn test_iswxdigit_special() {
        assert_eq!(iswxdigit(0x20), 0); // space
        assert_eq!(iswxdigit(-1), 0); // WEOF
    }
}
