// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::core::ffi::VaList;
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
/// Writes formatted output to the given file stream. This is a wrapper around
/// [`crate::vdprintf::vdprintf`] for the stream's file descriptor, and it sets the stream's error
/// indicator on failure.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `ap`: Variable argument list matching the format specifiers in `fmt`.
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
/// - `ap` provides arguments matching the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/vfprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vfprintf(stream: *mut FILE, fmt: *const c_char, ap: VaList<'_>) -> c_int {
    if stream.is_null() {
        return -1;
    }
    let fd: c_int = (*stream).fd;
    // SAFETY: forwarding the argument list to vdprintf with the stream's file descriptor.
    let ret: c_int = unsafe { crate::vdprintf::vdprintf(fd, fmt, ap) };
    if ret < 0 {
        // Reflect the failure (write error or invalid format) in the stream's error indicator
        // so ferror() reports it.
        (*stream).error = 1;
    }
    ret
}
