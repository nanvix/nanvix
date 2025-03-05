// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    ffi::{
        c_char,
        c_void,
    },
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn dlclose(_handle: *mut c_void) -> i32 {
    // TODO: https://github.com/nanvix/nanvix/issues/381
    ::nvx::error!("dlclose(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    -1
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn dlerror() -> *mut c_char {
    // TODO: https://github.com/nanvix/nanvix/issues/373
    ::nvx::error!("dlerror(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    ::core::ptr::null_mut()
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn dlopen(_filename: *const c_char, _flags: i32) -> *mut c_void {
    // TODO: https://github.com/nanvix/nanvix/issues/380
    ::nvx::error!("dlopen(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    ::core::ptr::null_mut()
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
    // TODO: https://github.com/nanvix/nanvix/issues/372
    ::nvx::error!("dlsym(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    ::core::ptr::null_mut()
}
