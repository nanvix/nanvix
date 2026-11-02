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
mod test;
