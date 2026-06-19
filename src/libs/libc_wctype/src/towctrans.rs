// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    towlower::towlower,
    towupper::towupper,
    wctrans::{
        WCTRANS_TOLOWER,
        WCTRANS_TOUPPER,
    },
    wctrans_t::wctrans_t,
    wint_t::wint_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Maps a wide character using a character mapping descriptor.
///
/// # Parameters
///
/// - `wc`: Wide character to map.
/// - `desc`: Character mapping descriptor returned by `wctrans()`.
///
/// # Return Value
///
/// The mapped wide character, or `wc` unchanged if the descriptor is zero or invalid.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn towctrans(wc: wint_t, desc: wctrans_t) -> wint_t {
    match desc {
        WCTRANS_TOLOWER => towlower(wc),
        WCTRANS_TOUPPER => towupper(wc),
        _ => wc,
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::towctrans;
    use crate::wctrans::{
        WCTRANS_TOLOWER,
        WCTRANS_TOUPPER,
    };

    #[test]
    fn test_towctrans_valid_mappings() {
        assert_eq!(towctrans(0x41, WCTRANS_TOLOWER), 0x61); // 'A' -> 'a'
        assert_eq!(towctrans(0x61, WCTRANS_TOUPPER), 0x41); // 'a' -> 'A'
    }

    #[test]
    fn test_towctrans_zero_descriptor() {
        assert_eq!(towctrans(0x41, 0), 0x41); // 'A'
    }

    #[test]
    fn test_towctrans_invalid_descriptor() {
        assert_eq!(towctrans(0x41, 999), 0x41); // 'A'
    }
}
