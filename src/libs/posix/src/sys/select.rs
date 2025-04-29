// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::{
    time::time_t,
    types::suseconds_t,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct timeval {
    /// Seconds.
    pub tv_sec: time_t,
    /// Nano-seconds.
    pub tv_nsec: suseconds_t,
}

impl From<timeval> for crate::time::timespec {
    fn from(tv: timeval) -> Self {
        Self {
            tv_sec: tv.tv_sec,
            tv_nsec: tv.tv_nsec,
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use crate::{
        errno::__errno_location,
        ffi::c_int,
    };
    use ::nvx::sys::error::ErrorCode;

    #[allow(clippy::missing_safety_doc)]
    #[no_mangle]
    pub extern "C" fn select(
        _nfds: c_int,
        _readfds: *mut c_int,
        _writefds: *mut c_int,
        _exceptfds: *mut c_int,
        _timeout: *mut c_int,
    ) -> c_int {
        // TODO: https://github.com/nanvix/nanvix/issues/468
        ::nvx::error!("select(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}
