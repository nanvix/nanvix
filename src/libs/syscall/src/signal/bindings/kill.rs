// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    errno::__errno_location,
    ffi::c_int,
    sys_types::pid_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub extern "C" fn kill(pid: pid_t, signal: c_int) -> c_int {
    ::syslog::trace!("kill(): pid ={pid}, signal={signal}");

    // TODO: Implement this system call.
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
