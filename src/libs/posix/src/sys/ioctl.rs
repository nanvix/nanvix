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
    use crate::ffi::c_int;

    #[allow(clippy::missing_safety_doc)]
    #[no_mangle]
    pub extern "C" fn ioctl(_fd: c_int, _request: c_int, _arg: *mut c_int) -> c_int {
        // TODO: https://github.com/nanvix/nanvix/issues/351
        ::syslog::error!("ioctl(): not implemented, ignoring");
        0
    }
}
