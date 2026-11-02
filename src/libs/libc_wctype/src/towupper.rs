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
mod test;
