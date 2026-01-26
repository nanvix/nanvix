// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    fcntl,
};
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
/// Renames a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor of the old file.
/// - `oldpath`:  Pathname of the old file.
/// - `newdirfd`: Directory file descriptor of the new file.
/// - `newpath`:  Pathname of the new file.
///
/// # Returns
///
/// Upon successful completion, the `renameat()` system call returns `0`. Otherwise, it returns
/// `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `oldpath` points to a valid null-terminated C string.
/// - `newpath` points to a valid null-terminated C string.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn renameat(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
) -> c_int {
    // Check if `oldpath` is null.
    if oldpath.is_null() {
        ::syslog::error!(
            "renameat(): oldpath is null (olddirfd={olddirfd:?}, newdirfd={newdirfd:?}, \
             oldpath={oldpath:?}, newpath={newpath:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `newpath` is null.
    if newpath.is_null() {
        ::syslog::error!(
            "renameat(): newpath is null (olddirfd={olddirfd:?}, newdirfd={newdirfd:?}, \
             oldpath={oldpath:?}, newpath={newpath:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `oldpath` to a Rust string.
    let old_pathname: &str = match ffi::CStr::from_ptr(oldpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!(
                "renameat(): invalid old pathname (olddirfd={olddirfd:?}, newdirfd={newdirfd:?}, \
                 oldpath={oldpath:?}, newpath={newpath:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `newpath` to a Rust string.
    let new_pathname: &str = match ffi::CStr::from_ptr(newpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!(
                "renameat(): invalid new pathname (olddirfd={olddirfd:?}, newdirfd={newdirfd:?}, \
                 oldpath={old_pathname:?}, newpath={newpath:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Execute system call and check the result.
    match fcntl::renameat(olddirfd, old_pathname, newdirfd, new_pathname) {
        // System call succeeded.
        Ok(()) => 0,
        // System call failed.
        Err(error) => {
            ::syslog::error!(
                "renameat(): {error:?} (olddirfd={olddirfd:?}, oldpath={old_pathname:?}, \
                 newdirfd={newdirfd:?}, newpath={new_pathname:?}, error={error:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
