// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::strtod::strtod;
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to `f64`.
///
/// This function is equivalent to `strtod(nptr, null)`.
///
/// # Parameters
///
/// - `nptr`: Pointer to the null-terminated string to be converted.
///
/// # Returns
///
/// The converted `f64` value. Returns `0.0` if no valid conversion could be performed.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that
/// `nptr` points to a valid null-terminated string.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/atof.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn atof(nptr: *const c_char) -> f64 {
    strtod(nptr, core::ptr::null_mut())
}

#[cfg(all(test, feature = "std"))]
mod tests;
