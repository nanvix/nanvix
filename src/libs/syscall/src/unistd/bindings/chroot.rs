// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the root directory of the calling process. The `chroot()` function changes the root
/// directory of the calling process to the directory specified by `path`. This directory will be
/// used as the starting point for pathnames beginning with `/`. The calling process's working
/// directory is unaffected by this call. This function is commonly used to create a "chrooted"
/// environment where processes are restricted to a specific directory tree, providing a form of
/// sandboxing and security isolation.
///
/// # Parameters
///
/// - `path`: Pathname of the new root directory. This must be a valid null-terminated string
///   specifying either an absolute or relative path to an existing directory. The calling process
///   must have appropriate permissions to access the specified directory and typically requires
///   superuser privileges to execute this operation.
///
/// # Returns
///
/// The `chroot()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error. Common error conditions include directory not found, permission denied,
/// path is not a directory, invalid path string, or insufficient privileges.
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
pub unsafe extern "C" fn chroot(path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/517
    ::syslog::debug!("chroot(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
