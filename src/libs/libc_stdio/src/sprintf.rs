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
/// Writes formatted output to `buf` with no bounds checking. This is a variadic wrapper around
/// [`crate::vsprintf::vsprintf`].
///
/// # Parameters
///
/// - `buf`: Pointer to the destination buffer. Must be large enough to hold the result.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `...`: Arguments matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters written (excluding the null terminator).
///
/// # Safety
///
/// This function is unsafe because it performs no bounds checking. The caller must ensure that:
/// - `buf` points to a writable buffer large enough to hold the formatted output.
/// - `fmt` points to a valid, null-terminated format string.
/// - The variadic arguments match the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sprintf(buf: *mut c_char, fmt: *const c_char, args: ...) -> c_int {
    // SAFETY: forwarding the variadic argument list to vsprintf.
    unsafe { crate::vsprintf::vsprintf(buf, fmt, args) }
}
