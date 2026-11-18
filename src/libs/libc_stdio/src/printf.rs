// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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
/// Writes formatted output to standard output. This is a variadic wrapper around
/// [`crate::vprintf::vprintf`].
///
/// # Parameters
///
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `...`: Arguments matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters written on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that:
/// - `fmt` points to a valid, null-terminated format string.
/// - The variadic arguments match the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/printf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn printf(fmt: *const c_char, args: ...) -> c_int {
    // SAFETY: forwarding the variadic argument list to vprintf.
    unsafe { crate::vprintf::vprintf(fmt, args) }
}
