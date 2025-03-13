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
pub type DIR = ();

// TODO: fix this to reflect the actual implementation.
pub type dirent = ();

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
    pub unsafe extern "C" fn closedir(_dirp: *mut DIR) -> c_int {
        // TODO: https://github.com/nanvix/nanvix/issues/518
        ::nvx::error!("closedir(): not implemented");
        unsafe {
            errno = ErrorCode::InvalidSysCall.into_errno();
        }
        -1
    }

    #[allow(clippy::missing_safety_doc)]
    #[no_mangle]
    pub unsafe extern "C" fn opendir(_name: *const u8) -> *mut DIR {
        // TODO: https://github.com/nanvix/nanvix/issues/520
        ::nvx::error!("opendir(): not implemented");
        unsafe {
            errno = ErrorCode::InvalidSysCall.into_errno();
        }
        core::ptr::null_mut()
    }

    #[allow(clippy::missing_safety_doc)]
    #[no_mangle]
    pub unsafe extern "C" fn readdir(_dirp: *mut DIR) -> *mut dirent {
        // TODO: https://github.com/nanvix/nanvix/issues/522
        ::nvx::error!("readdir(): not implemented");
        unsafe {
            errno = ErrorCode::InvalidSysCall.into_errno();
        }
        core::ptr::null_mut()
    }
}
