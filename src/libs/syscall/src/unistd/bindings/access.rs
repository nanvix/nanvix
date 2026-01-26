// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::{
        c_char,
        c_int,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks user's permissions for a file. The `access()` function checks whether the calling
/// process can access the file specified by `path` in the manner specified by `mode`. This
/// function uses the real user ID and real group ID of the calling process (rather than the
/// effective IDs) to perform the access check. The `access()` function is a convenience wrapper
/// around `faccessat()` that operates relative to the current working directory, providing
/// backward compatibility with traditional UNIX systems.
///
/// # Parameters
///
/// - `path`: Pathname of the file to check for accessibility. This must be a valid null-terminated
///   string specifying either an absolute or relative path to the file.
/// - `mode`: Specifies the accessibility checks to perform. This is a bitwise OR of one or more
///   of `F_OK` (test for file existence), `R_OK` (test for read permission), `W_OK` (test for
///   write permission), and `X_OK` (test for execute permission).
///
/// # Returns
///
/// The `access()` function returns `0` if all requested access modes are permitted. On error,
/// it returns `-1` and sets `errno` to indicate the error. Common error conditions include
/// file not found, permission denied, or invalid path string.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - `path` remains valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn access(path: *const c_char, mode: c_int) -> c_int {
    unistd::bindings::faccessat::faccessat(AT_FDCWD, path, mode, 0)
}
