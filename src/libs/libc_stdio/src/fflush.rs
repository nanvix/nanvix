// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Flushes any buffered output for the stream pointed to by `stream`. Since this implementation
/// uses unbuffered I/O, this function is a no-op.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream, or null to flush all open streams.
///
/// # Returns
///
/// Zero on success.
///
/// # Safety
///
/// This function is unsafe for API compatibility. When `stream` is non-null the caller must
/// ensure that it points to a valid [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fflush.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fflush(_stream: *mut FILE) -> c_int {
    // No-op: this implementation uses unbuffered I/O.
    0
}
