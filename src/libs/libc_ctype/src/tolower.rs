// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts an uppercase letter to its lowercase equivalent.
///
/// # Parameters
///
/// - `c`: Character to convert, as an `int`.
///
/// # Return Value
///
/// The lowercase equivalent if `c` is an uppercase letter ('A'-'Z'), otherwise `c` unchanged.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tolower(c: c_int) -> c_int {
    if (0x41..=0x5A).contains(&c) {
        c + 32
    } else {
        c
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
