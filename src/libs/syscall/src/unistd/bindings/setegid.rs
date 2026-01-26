// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::gid_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the effective group ID of the calling process.
///
/// # Parameters
///
/// - `gid`: New group ID.
///
/// # Returns
///
/// Upon successful completion, `setegid()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global variables.
///
/// This function is safe to use if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setegid(gid: gid_t) -> c_int {
    ::syslog::error!("setegid(): gid={gid:?}");

    // Check whether `gid` equals to the effective group ID of the calling process.
    match unistd::getegid() {
        Ok(egid) if gid == egid => 0,
        Ok(egid) => {
            ::syslog::error!("setegid(): operation not permitted (gid={gid:?}, egid={egid:?})");
            *__errno_location() = ErrorCode::OperationNotPermitted.get();
            -1
        },
        Err(error) => {
            ::syslog::error!("setegid(): {error:?} (gid={gid:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
