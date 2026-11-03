// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::{
    wint_t,
    WEOF,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a single-byte character to a wide character. In the POSIX locale, every byte value in
/// the range 0..=255 is a valid single-byte character.
///
/// # Parameters
///
/// - `c`: Character to convert, as an `int`.
///
/// # Return Value
///
/// The wide character representation of `c`, or `WEOF` if `c` is `EOF` or outside the byte range.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn btowc(c: c_int) -> wint_t {
    if (0..=255).contains(&c) {
        c as wint_t
    } else {
        WEOF
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
