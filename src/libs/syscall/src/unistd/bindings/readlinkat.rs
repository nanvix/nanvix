// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::core::{
    ffi,
    slice,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    limits::PATH_MAX,
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the value of a symbolic link relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`: Path to the symbolic link.
/// - `buf`: Buffer to store the value of the symbolic link.
/// - `bufsize`: Size of the buffer.
///
/// # Returns
///
/// Upon successful completion, `readlinkat()` returns the number of bytes read. Otherwise, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - `buf` points to a valid memory location of `bufsize` bytes.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readlinkat(
    dirfd: c_int,
    path: *const c_char,
    buf: *mut c_char,
    bufsize: c_size_t,
) -> c_ssize_t {
    // Attempt to convert `buf`.
    let buf: &mut [u8] = {
        // Check if `bufsize` is invalid.
        let bufsize: usize = if (bufsize == 0) || (bufsize as usize > PATH_MAX) {
            ::syslog::trace!(
                "readlinkat(): invalid buffer size (dirfd={dirfd:?}, path={path:?}, buf={buf:?}, \
                 bufsize={bufsize:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        } else {
            bufsize as usize
        };

        // Check if `buf` is invalid.
        if buf.is_null() {
            ::syslog::trace!(
                "readlinkat(): invalid buffer (dirfd={dirfd:?}, path={path:?}, buf={buf:?}, \
                 bufsize={bufsize:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Attempt to convert `path`.
        slice::from_raw_parts_mut(buf as *mut u8, bufsize)
    };

    // Attempt to convert `path`.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_error) => {
            ::syslog::trace!(
                "readlinkat(): invalid path (dirfd={dirfd:?}, path={path:?}, buf={buf:?}, \
                 bufsize={bufsize:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Read symbolic link and parse the result.
    match unistd::readlinkat(dirfd, path, buf) {
        Ok(bytes_read) => bytes_read,
        Err(error) => {
            ::syslog::trace!(
                "readlinkat(): {error:?}, (dirfd={dirfd:?}, path={path:?}, buf={buf:?}, \
                 bufsize={bufsize:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
