// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes formatted output to the given file stream. This is a variadic wrapper around
/// [`crate::vfprintf::vfprintf`].
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `...`: Arguments matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters written on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `stream` points to a valid, open [`FILE`] structure.
/// - `fmt` points to a valid, null-terminated format string.
/// - The variadic arguments match the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fprintf(stream: *mut FILE, fmt: *const c_char, args: ...) -> c_int {
    // SAFETY: forwarding the variadic argument list to vfprintf.
    unsafe { crate::vfprintf::vfprintf(stream, fmt, args) }
}
