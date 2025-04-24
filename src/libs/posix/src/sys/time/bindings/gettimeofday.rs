// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    ffi::{
        c_int,
        c_void,
    },
    sys::time::timeval,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn gettimeofday(_tp: *mut timeval, _tzp: *mut c_void) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/317
    ::nvx::error!("gettimeofday(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}
