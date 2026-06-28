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
    ffi::c_char,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Opens a process by creating a pipe, forking, and invoking the shell.
///
/// # Parameters
///
/// - `command`: Null-terminated string containing the command to be executed.
/// - `mode`: Null-terminated string that specifies the mode for the pipe (e.g., "r" or "w").
///
/// # Returns
///
/// On success, returns a non-null pointer to an opaque stream object. On failure, returns a null
/// pointer and sets `errno` to indicate the error.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// A future implementation should create the appropriate pipe, fork a child process, and execute
/// the requested command in a POSIX-compatible shell environment.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers.
/// It is safe to call this function if `command` and `mode` (when non-null) point to valid
/// null-terminated C strings.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/popen.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn popen(_command: *const c_char, _mode: *const c_char) -> *mut FILE {
    // SAFETY: `__errno_location()` returns a valid pointer to the errno storage.
    unsafe { *__errno_location() = ENOSYS };
    core::ptr::null_mut()
}
