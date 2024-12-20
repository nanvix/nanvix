// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::fcntl;
use ::core::ffi;
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn open(path: *const i8, flags: i32, mode: u32) -> i32 {
    let path: &str = match unsafe { ffi::CStr::from_ptr(path).to_str() } {
        Ok(pathname) => pathname,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };

    fcntl::openat(fcntl::AT_FDCWD, path, flags, mode)
}
