// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syscall::sys::resource::rlimit;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getrlimit(_resource: c_int, _rlim: *mut rlimit) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/459
    ::syslog::error!("getrlimit(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setrlimit(_resource: c_int, _rlim: *const rlimit) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/469
    ::syslog::error!("setrlimit(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
