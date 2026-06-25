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
/// Formats output according to `fmt` and the trailing variadic arguments and writes it directly to
/// the file descriptor `fd`. Unlike [`fprintf`](`crate::fprintf::fprintf`) it operates on a raw
/// descriptor rather than a [`FILE`](`crate::streams::FILE`) stream.
///
/// # Parameters
///
/// - `fd`: Destination file descriptor.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `...`: Arguments matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of bytes written on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `fd` is a valid, writable file descriptor.
/// - `fmt` points to a valid, null-terminated format string.
/// - The variadic arguments match the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/dprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn dprintf(fd: c_int, fmt: *const c_char, args: ...) -> c_int {
    // SAFETY: forwarding the variadic argument list to vdprintf.
    unsafe { crate::vdprintf::vdprintf(fd, fmt, args) }
}
