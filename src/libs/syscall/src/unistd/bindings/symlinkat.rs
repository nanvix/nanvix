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
/// Creates a symbolic link relative to a directory file descriptor.
///
/// # Parameters
///
/// - `target`: Path to the file to be linked.
/// - `dirfd`: Directory file descriptor.
/// - `linkpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `target` points to a valid null-terminated string.
/// - `linkpath` points to a valid null-terminated string.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symlinkat(
    target: *const c_char,
    dirfd: c_int,
    linkpath: *const c_char,
) -> c_int {
    ::syslog::error!("symlinkat(): target={target:?}, dirfd={dirfd}, linkpath={linkpath:?}",);

    // Attempt to convert `target`.
    let target: &str = {
        // Check if `target` is invalid.
        if target.is_null() {
            ::syslog::error!(
                "symlinkat(): target is null (target={target:?}, dirfd={dirfd}, \
                 linkpath={linkpath:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        match ffi::CStr::from_ptr(target).to_str() {
            Ok(pathname) => pathname,
            Err(_) => {
                ::syslog::error!("symlinkat(): invalid target");
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        }
    };

    // Attempt to convert `linkpath`.
    let linkpath: &str = {
        // Check if `linkpath` is invalid.
        if linkpath.is_null() {
            ::syslog::error!(
                "symlinkat(): linkpath is null (target={target:?}, dirfd={dirfd}, \
                 linkpath={linkpath:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        match ffi::CStr::from_ptr(linkpath).to_str() {
            Ok(pathname) => pathname,
            Err(_) => {
                ::syslog::error!("symlinkat(): invalid linkpath");
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        }
    };

    // Create symbolic link and parse the result.
    match unistd::symlinkat(target, dirfd, linkpath) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "symlinkat(): {error:?} (target={target:?}, dirfd={dirfd}, linkpath={linkpath:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
