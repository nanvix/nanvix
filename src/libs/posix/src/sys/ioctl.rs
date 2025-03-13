// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use crate::{
        errno::errno,
        ffi::c_int,
    };
    use ::nvx::sys::error::ErrorCode;

    #[allow(clippy::missing_safety_doc)]
    #[no_mangle]
    pub extern "C" fn ioctl(_fd: c_int, _request: c_int, _arg: *mut c_int) -> c_int {
        // TODO: https://github.com/nanvix/nanvix/issues/351
        ::nvx::error!("ioctl(): not implemented");
        unsafe {
            errno = ErrorCode::InvalidSysCall.into_errno();
        }
        -1
    }
}
