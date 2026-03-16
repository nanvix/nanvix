// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    safe::sys::{
        config::SysConfigName,
        System,
    },
};
use ::sysapi::ffi::{
    c_int,
    c_long,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the value of a system configuration variable.
///
/// # Parameters
///
/// - `name` - Name of the system configuration variable to query.
///
/// # Return Value
///
/// On success, this function returns the value of the specified system configuration variable.
/// Otherwise, it returns -1 and sets `errno` to indicate the error.
///
/// # Errors
///
/// - [`sys::error::ErrorCode::InvalidArgument`] if the specified system configuration name is invalid.
/// - [`sys::error::ErrorCode::OperationNotSupported`] if the specified system configuration name is not
///   supported.
/// - [`sys::error::ErrorCode::ValueOutOfRange`] if the value of the specified system configuration variable
///   cannot be represented by the return type.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub extern "C" fn sysconf(name: c_int) -> c_long {
    // Attempt to convert `name` to `SysConfigName`.
    let name: SysConfigName = match SysConfigName::try_from(name) {
        Ok(name) => name,
        Err(error) => {
            ::syslog::trace!("sysconf(): {error:?}");
            unsafe {
                *__errno_location() = error.code.get();
            }
            return -1;
        },
    };

    // Attempt to get system configuration value and check for errors.
    match System::config(name) {
        Ok(value) => value.into(),
        Err(error) => {
            ::syslog::trace!("sysconf(): {error:?}");
            unsafe {
                *__errno_location() = error.code.get();
            }
            -1
        },
    }
}
