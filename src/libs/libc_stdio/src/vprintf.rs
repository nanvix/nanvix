// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::VaList;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// File descriptor for standard output.
const STDOUT_FD: c_int = 1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes formatted output to standard output. This is a wrapper around
/// [`crate::vdprintf::vdprintf`] for the standard output file descriptor.
///
/// # Parameters
///
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `ap`: Variable argument list matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters written on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that:
/// - `fmt` points to a valid, null-terminated format string.
/// - `ap` provides arguments matching the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/vprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vprintf(fmt: *const c_char, ap: VaList<'_>) -> c_int {
    // SAFETY: forwarding the argument list to vdprintf with the standard output descriptor.
    unsafe { crate::vdprintf::vdprintf(STDOUT_FD, fmt, ap) }
}
