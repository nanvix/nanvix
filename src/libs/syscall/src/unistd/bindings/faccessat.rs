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
/// Checks the accessibility of a file relative to a directory file descriptor. The `faccessat()`
/// function checks whether the calling process can access the file specified by `path` in the
/// manner specified by `mode`. If `path` is a relative pathname, it is interpreted relative to
/// the directory referred to by the file descriptor `dirfd`. If `path` is absolute, `dirfd` is
/// ignored. This function is useful for checking file permissions before attempting to open or
/// manipulate a file, and provides more flexibility than the traditional `access()` function.
///
/// # Parameters
///
/// - `dirfd`: File descriptor referring to a directory, relative to which `path` should be
///   interpreted if it is a relative pathname. Special value `AT_FDCWD` can be used to indicate
///   the current working directory.
/// - `path`: Pathname of the file to check for accessibility. This must be a valid null-terminated
///   string. Can be either absolute or relative to `dirfd`.
/// - `mode`: Specifies the accessibility checks to perform. This is a bitwise OR of one or more
///   of `F_OK` (file existence), `R_OK` (read permission), `W_OK` (write permission), and
///   `X_OK` (execute permission).
/// - `flag`: Flags that modify the behavior of the access check. Common flags include
///   `AT_EACCESS` to use effective user and group IDs instead of real IDs for the access check.
///
/// # Returns
///
/// The `faccessat()` function returns `0` if all requested access modes are permitted. On error,
/// it returns `-1` and sets `errno` to indicate the error. Common error conditions include
/// file not found, permission denied, invalid path, or invalid file descriptor.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - `path` remains valid for the duration of the function call.
/// - `dirfd` refers to a valid directory file descriptor or is `AT_FDCWD`.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn faccessat(
    dirfd: c_int,
    path: *const c_char,
    mode: c_int,
    flag: c_int,
) -> c_int {
    // Check if `path` is invalid.
    if path.is_null() {
        ::syslog::error!(
            "faccessat(): null path pointer (dirfd={dirfd:?}, path={path:?}, mode={mode:?}, \
             flag={flag:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `path`.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!(
                "faccessat(): invalid path (dirfd={dirfd:?}, path={path:?}, mode={mode:?}, \
                 flag={flag:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to check access permissions and check for errors.
    match crate::unistd::faccessat(dirfd, path, mode, flag) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "faccessat(): {error:?} (dirfd={dirfd:?}, path={path:?}, mode={mode:?}, \
                 flag={flag:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
