// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    errno::__errno_location,
    ffi::c_char,
    sys_stat,
};
use ::syslog::trace_syscall;
use sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Obtains information about the file named `pathname`.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
/// - `statbuf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::sys::stat::stat`]
///
/// # Safety
///
/// This function has undefined because it dereferences a raw pointer (ie. `statbuf`).
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn stat(pathname: *const c_char, statbuf: *mut sys_stat::stat) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match core::ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::trace!("stat(): invalid pathname");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let statbuf: &mut sys_stat::stat = &mut *statbuf;

    match crate::sys::stat::stat(pathname, statbuf) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::trace!(
                "stat(): failed (pathname={}, statbuf={:p}, error={:?})",
                pathname,
                statbuf,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
