// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syscall::sys::utsname;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Get information of the current system.
///
/// # Parameters
///
/// - `name`: Storage location for the system information.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, `-1`` is returned and `errno` is set to indicate the
/// error.
///
/// # Safety
///
/// This function is unsafe because it may deference raw pointers.
///
/// It is safe to use this function if and only if all the following conditions are met:
///
/// - The `name` points to a valid [`utsname`] structure.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn uname(name: *mut utsname::utsname) -> c_int {
    // Check if name is not valid.
    if name.is_null() {
        ::syslog::error!("uname(): name is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Execute system call and check for errors.
    match utsname::uname() {
        // Success, copy data to user buffer.
        Ok(name_) => {
            *name = name_;
            0
        },
        // Error, set errno.
        Err(error) => {
            *__errno_location() = error.code.get();
            -1
        },
    }
}
