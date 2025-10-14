// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_uint;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub extern "C" fn sleep(seconds: c_uint) -> c_uint {
    ::syslog::trace!("sleep(): seconds={seconds:?}");

    // TODO: https://github.com/nanvix/nanvix/issues/453
    ::syslog::debug!("sleep(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    0
}
