// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Types
//==================================================================================================

// TODO: fix this to reflect the actual implementation.
pub type timeval = ();

// TODO: fix this to reflect the actual implementation.
pub type timezone = ();

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use super::*;
    use crate::{
        errno::errno,
        ffi::c_int,
    };
    use ::nvx::sys::error::ErrorCode;

    #[allow(clippy::missing_safety_doc)]
    #[no_mangle]
    pub unsafe extern "C" fn gettimeofday(_tp: *mut timeval, _tzp: *mut timezone) -> c_int {
        // TODO: https://github.com/nanvix/nanvix/issues/317
        ::nvx::error!("gettimeofday(): not implemented");
        unsafe {
            errno = ErrorCode::InvalidSysCall.into_errno();
        }
        -1
    }
}
