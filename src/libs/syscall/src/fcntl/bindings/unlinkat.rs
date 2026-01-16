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
/// Unlinks a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the file.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, `unlinkat()` returns zero. Otherwise, it returns -1 and sets `errno`
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and access global variables.
///
/// It is safe to call this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
/// - `pathname` points to a valid null-terminated C string.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unlinkat(dirfd: c_int, pathname: *const c_char, flags: c_int) -> c_int {
    // Check if `pathname` is null.
    if pathname.is_null() {
        ::syslog::error!(
            "unlinkat(): null pathname (dirfd={dirfd:?}, pathname={pathname:?}, flags={flags:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `pathname` to a Rust string.
    let path: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!(
                "unlinkat(): invalid pathname (dirfd={dirfd:?}, pathname={pathname:?}, \
                 flags={flags:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Execute system call and check the result.
    match fcntl::unlinkat(dirfd, path, flags) {
        // System call succeeded.
        Ok(()) => 0,
        // System call failed.
        Err(error) => {
            ::syslog::error!(
                "unlinkat(): {error:?} (dirfd={dirfd:?}, pathname={pathname:?}, flags={flags:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
