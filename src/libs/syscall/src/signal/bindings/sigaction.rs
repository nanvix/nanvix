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
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
#[trace_syscall]
pub extern "C" fn sigaction(
    signum: c_int,
    act: *const sigaction_t,
    oldact: *mut sigaction_t,
) -> c_int {
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
