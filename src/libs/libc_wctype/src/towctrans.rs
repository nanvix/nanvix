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
mod test;
