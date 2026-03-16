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
/// Opens the file specified by `pathname`.
///
/// # Parameters
///
/// - `path`:  Pathname of the file to open.
/// - `flags`: Flags to open the file.
/// - `mode`:  Mode of the file.
///
/// # Returns
///
/// Upon successful completion, the `open()` system call returns a non-negative integer representing
/// the lowest numbered unused file descriptor. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may deference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `path` points to a valid null-terminated C string.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    // Check if `path` is null.
    if path.is_null() {
        ::syslog::trace!(
            "open(): null path pointer (path={path:?}, flags={flags:?}, mode={mode:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `path`.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::trace!(
                "open(): invalid pathname (path={path:?}, flags={flags:?}, mode={mode:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Run system call and check for errors.
    match fcntl::open(pathname, flags, mode) {
        Ok(fd) => fd,
        Err(error) => {
            ::syslog::trace!("open(): {error:?} (path={path:?}, flags={flags:?}, mode={mode:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
