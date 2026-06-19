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
/// Tests whether a wide character is a punctuation character.
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is punctuation, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswpunct(wc: wint_t) -> c_int {
    if (0x21..=0x2F).contains(&wc)
        || (0x3A..=0x40).contains(&wc)
        || (0x5B..=0x60).contains(&wc)
        || (0x7B..=0x7E).contains(&wc)
    {
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
    use super::iswpunct;

    #[test]
    fn test_iswpunct_punctuation() {
        assert_ne!(iswpunct(0x21), 0); // '!'
        assert_ne!(iswpunct(0x2F), 0); // '/'
        assert_ne!(iswpunct(0x3A), 0); // ':'
        assert_ne!(iswpunct(0x40), 0); // '@'
        assert_ne!(iswpunct(0x5B), 0); // '['
        assert_ne!(iswpunct(0x60), 0); // '`'
        assert_ne!(iswpunct(0x7B), 0); // '{'
        assert_ne!(iswpunct(0x7E), 0); // '~'
    }

    #[test]
    fn test_iswpunct_non_punct() {
        assert_eq!(iswpunct(0x41), 0); // 'A'
        assert_eq!(iswpunct(0x30), 0); // '0'
        assert_eq!(iswpunct(0x20), 0); // space
    }

    #[test]
    fn test_iswpunct_special() {
        assert_eq!(iswpunct(0x00), 0); // null
        assert_eq!(iswpunct(-1), 0); // WEOF
    }
}
