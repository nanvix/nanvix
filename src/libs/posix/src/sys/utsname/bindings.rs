// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ffi::c_int,
    sys::utsname::{
        syscall,
        utsname,
    },
};
use nvx::sys::error::ErrorCode;

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
pub unsafe extern "C" fn uname(name: *mut utsname) -> c_int {
    // Check if name is not valid.
    if name.is_null() {
        ::syslog::error!("uname(): name is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Execute system call and check for errors.
    match syscall::uname() {
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
