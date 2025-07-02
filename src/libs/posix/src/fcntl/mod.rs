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
use ::syscall::fcntl;

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
/// Upon successful completion, `unlinkat()` returns zero. Otherwise, it returns -1 and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `pathname` points to a valid null-terminated C string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unlinkat(dirfd: c_int, pathname: *const c_char, flags: c_int) -> c_int {
    ::syslog::trace!("unlinkat(): dirfd={:?}, pathname={:?}, flags={:?}", dirfd, pathname, flags);

    // Attempt to convert `pathname` to a Rust string.
    let path: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("unlinkat(): invalid pathname (dirfd={:?}, flags={:?})", dirfd, flags);
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
                "unlinkat(): failed (dirfd={:?}, pathname={:?}, flags={:?}, error={:?})",
                dirfd,
                path,
                flags,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
