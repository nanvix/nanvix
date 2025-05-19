// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::nvx::sys::error::ErrorCode;
use ::syscall::{
    ffi::{
        c_int,
        c_void,
    },
    sys::time::timeval,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gettimeofday(_tp: *mut timeval, _tzp: *mut c_void) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/317
    ::syslog::error!("gettimeofday(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}
