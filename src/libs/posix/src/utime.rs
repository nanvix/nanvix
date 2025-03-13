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
pub type utimbuf = ();

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
    pub unsafe extern "C" fn utime(_filename: *const c_int, _times: *const utimbuf) -> c_int {
        // TODO: https://github.com/nanvix/nanvix/issues/524
        ::nvx::error!("utime(): not implemented");
        unsafe {
            errno = ErrorCode::InvalidSysCall.into_errno();
        }
        -1
    }
}
