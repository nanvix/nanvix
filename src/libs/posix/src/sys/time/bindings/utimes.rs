// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    ffi::{
        c_char,
        c_int,
    },
    sys::time::timeval,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn utimes(_filename: *const c_char, _times: *const timeval) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/524
    ::nvx::error!("utime(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}
