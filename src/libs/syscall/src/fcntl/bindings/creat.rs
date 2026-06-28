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
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::mode_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new file or rewrites an existing one. It is equivalent to calling `open()` with the
/// flags `O_WRONLY | O_CREAT | O_TRUNC`.
///
/// # Parameters
///
/// - `path`: Pathname of the file to create.
/// - `mode`: Permission bits applied when the file is created.
///
/// # Returns
///
/// Upon successful completion returns a non-negative file descriptor. Otherwise returns `-1` and
/// sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. It is safe to call when `path`
/// points to a valid, null-terminated C string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/creat.html>
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn creat(path: *const c_char, mode: mode_t) -> c_int {
    // Check if `path` is null.
    if path.is_null() {
        ::syslog::trace!("creat(): null path pointer (path={path:?}, mode={mode:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `path`.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::trace!("creat(): invalid pathname (path={path:?}, mode={mode:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Run system call and check for errors.
    match fcntl::creat(pathname, mode) {
        Ok(fd) => fd,
        Err(error) => {
            ::syslog::warn!("creat(): {error:?} (path={path:?}, mode={mode:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
