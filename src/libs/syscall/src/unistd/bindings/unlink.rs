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
/// Deletes a name from the filesystem.
///
/// # Parameters
///
/// - `path`: Path to the file to be unlinked.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::unlink()`]
///
#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
pub unsafe extern "C" fn unlink(path: *const c_char) -> c_int {
    // Attempt to convert `path`.
    let path: &str = {
        // Check if `path` is invalid.
        if path.is_null() {
            ::syslog::error!("unlink(): path is null (path={path:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        match ffi::CStr::from_ptr(path).to_str() {
            Ok(pathname) => pathname,
            Err(_) => {
                ::syslog::error!("unlink(): invalid path (path={path:?})");
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        }
    };

    // Process system call and parse result.
    match unistd::unlink(path) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("unlink(): {error:?} (path={path:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
