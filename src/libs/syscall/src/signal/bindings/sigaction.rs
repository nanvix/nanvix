// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::signal::sigaction_t;
use ::sys::error::ErrorCode;
use ::sysapi::{
    errno::__errno_location,
    ffi::c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub extern "C" fn sigaction(
    signum: c_int,
    act: *const sigaction_t,
    oldact: *mut sigaction_t,
) -> c_int {
    ::syslog::trace!("sigaction(): signum={signum}, act={act:p}, oldact = {oldact:p}");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
