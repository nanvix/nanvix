// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    time::timespec,
};
use ::syslog::trace_syscall;
use sysapi::errno::__errno_location;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nanosleep(req: *const timespec, rem: *mut timespec) -> c_int {
    // Check if `req` is valid.
    if req.is_null() {
        ::syslog::error!("nanosleep(): invalid req pointer");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    let req: &timespec = unsafe { &*req };

    let mut rem: Option<&mut timespec> = if rem.is_null() {
        None
    } else {
        Some(unsafe { &mut *rem })
    };

    // Sleep for the requested time and check for errors.
    match crate::time::nanosleep(req, &mut rem) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "nanosleep(): failed (req={:?}, rem={:?}, error={:?})",
                req,
                rem,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
