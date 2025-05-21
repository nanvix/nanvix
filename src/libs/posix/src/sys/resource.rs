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
pub type rlimit = ();

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use super::*;
    use crate::{
        errno::__errno_location,
        ffi::c_int,
    };
    use ::sys::error::ErrorCode;

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
}
