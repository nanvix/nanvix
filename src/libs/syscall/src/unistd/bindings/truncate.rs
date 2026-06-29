// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::off_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Truncates a file to a specified length using a pathname. The `truncate()` function causes the
/// regular file named by `path` to be truncated to a length of exactly `length` bytes. If the file
/// was previously larger than `length`, the extra data is discarded. If the file was previously
/// shorter than `length`, it is extended with null bytes (`\0`). This function is similar to
/// `ftruncate()` but operates on a pathname instead of an open file descriptor.
///
/// # Parameters
///
/// - `path`: Path to the file to be truncated.
/// - `length`: New length of the file in bytes.
///
/// # Returns
///
/// Upon successful completion, `truncate()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
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
pub unsafe extern "C" fn truncate(path: *const c_char, length: off_t) -> c_int {
    // Check if `path` is invalid.
    if path.is_null() {
        ::syslog::warn!("truncate(): path is null (path={path:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `path`.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(path) => path,
        Err(_) => {
            ::syslog::warn!("truncate(): invalid path (path={path:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to truncate the file and check for errors.
    match crate::unistd::truncate(path, length) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::warn!("truncate(): {error:?} (path={path:?}, length={length:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
