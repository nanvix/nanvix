// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
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
/// Creates a symbolic link named `linkpath` which contains the string `target`.
///
/// # Parameters
///
/// - `target`: Path to the file to be linked.
/// - `linkpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::syscall::symlink()`]
///
#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int {
    // Attempt to convert `target`.
    let target: &str = {
        // Check if `target` is invalid.
        if target.is_null() {
            syslog::error!(
                "symlink(): target path is null (target={target:?}, linkpath={linkpath:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        match ffi::CStr::from_ptr(target).to_str() {
            Ok(pathname) => pathname,
            Err(_) => {
                ::syslog::error!(
                    "symlink(): invalid target (target={target:?}, linkpath={linkpath:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        }
    };

    // Attempt to convert `linkpath`.
    let linkpath: &str = {
        // Check if `linkpath` is invalid.
        if linkpath.is_null() {
            syslog::error!(
                "symlink(): linkpath is null (target={target:?}, linkpath={linkpath:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        match ffi::CStr::from_ptr(linkpath).to_str() {
            Ok(pathname) => pathname,
            Err(_) => {
                ::syslog::error!(
                    "symlink(): invalid linkpath (target={target:?}, linkpath={linkpath:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        }
    };

    // Check if the system call failed.
    match unistd::symlink(target, linkpath) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("symlink(): {error:?} (target={target}, linkpath={linkpath})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
