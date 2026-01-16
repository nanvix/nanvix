// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi;
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
/// Changes the current working directory of the calling process. The `chdir()` function causes
/// the directory specified by `path` to become the current working directory, which is the
/// starting point for path searches for pathnames that do not begin with `/`. This function
/// affects the entire process and all subsequent relative path operations will be resolved
/// relative to the new working directory. The change persists until the process terminates
/// or another `chdir()` call is made.
///
/// # Parameters
///
/// - `path`: Pathname of the new working directory. This must be a valid null-terminated string
///   specifying either an absolute or relative path to an existing directory. The calling process
///   must have search permission for the specified directory.
///
/// # Returns
///
/// The `chdir()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
/// indicate the error. Common error conditions include directory not found, permission denied,
/// path is not a directory, or invalid path string.
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
pub unsafe extern "C" fn chdir(path: *const c_char) -> c_int {
    // Check if `path` is invalid.
    if path.is_null() {
        ::syslog::error!("chdir(): path is null (path={path:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `path`.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("chdir(): invalid path (path={path:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to change the current working directory and check for errors.
    match crate::unistd::chdir(path) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("chdir(): {error:?} (path={path:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
