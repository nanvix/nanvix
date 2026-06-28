// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::{
    errno::{
        __errno_location,
        ENOSYS,
    },
    ffi::c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Closes a stream opened by [`crate::popen::popen`] and waits for the associated process to
/// terminate.
///
/// # Parameters
///
/// - `stream`: Pointer previously returned by [`crate::popen::popen`].
///
/// # Returns
///
/// On success, returns the termination status of the command. On failure, returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// A future implementation should close any pipe file descriptors, reap the child process, and
/// return its status code in a POSIX-compatible manner.
///
/// # Safety
///
/// This function is unsafe because it operates on an opaque raw pointer supplied by foreign
/// callers. It is safe to call this function if `stream` is either null or a value previously
/// returned by [`crate::popen::popen`] in a future, fully implemented version.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/pclose.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn pclose(_stream: *mut FILE) -> c_int {
    // SAFETY: `__errno_location()` returns a valid pointer to the errno storage.
    unsafe { *__errno_location() = ENOSYS };
    -1
}
