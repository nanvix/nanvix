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
/// Tests whether a wide character is a graphic character (printable, non-space).
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is graphic, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswgraph(wc: wint_t) -> c_int {
    if (0x21..=0x7E).contains(&wc) {
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
    use super::iswgraph;
    use crate::wint_t::wint_t;

    #[test]
    fn test_iswgraph_graphic() {
        for c in 0x21_i32..=0x7E_i32 {
            assert_ne!(iswgraph(c as wint_t), 0);
        }
    }

    #[test]
    fn test_iswgraph_space() {
        assert_eq!(iswgraph(0x20), 0); // space is not graphic
    }

    #[test]
    fn test_iswgraph_control() {
        for c in 0x00_i32..=0x1F_i32 {
            assert_eq!(iswgraph(c as wint_t), 0);
        }
        assert_eq!(iswgraph(0x7F), 0); // DEL
    }

    #[test]
    fn test_iswgraph_special() {
        assert_eq!(iswgraph(-1), 0); // WEOF
    }
}
