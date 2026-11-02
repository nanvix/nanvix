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
/// Tests whether a wide character is a blank character (space or tab).
///
/// # Parameters
///
/// - `wc`: Wide character to test.
///
/// # Return Value
///
/// Non-zero if the character is blank, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iswblank(wc: wint_t) -> c_int {
    if wc == 0x20 || wc == 0x09 {
        1
    } else {
        0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
