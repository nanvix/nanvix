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
/// Converts a character to a 7-bit ASCII value by clearing the high-order bits (POSIX extension).
///
/// Unlike `tolower`/`toupper`, this function intentionally masks *every* input instead of
/// preserving out-of-range values: `EOF` and values outside `0..=0xFF` are also reduced to their
/// low 7 bits. This matches the POSIX/NewLib definition of `toascii` (`c & 0x7F`).
///
/// # Parameters
///
/// - `c`: Character to convert, as an `int`.
///
/// # Return Value
///
/// The value of `c` with only the lower 7 bits preserved (`c & 0x7F`).
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn toascii(c: c_int) -> c_int {
    c & 0x7F
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
