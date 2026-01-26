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
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the real group ID of the calling process.
///
/// # Parameters
///
/// - `gid`: New group ID.
///
/// # Returns
///
/// Upon successful completion, `setgid()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global variables.
///
/// This function is safe to use if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgid(gid: gid_t) -> c_int {
    ::syslog::error!("setgid(): gid={gid:?}");

    // Check whether `gid` equals to the real group ID of the calling process.
    match unistd::getgid() {
        Ok(rgid) if gid == rgid => 0,
        Ok(rgid) => {
            ::syslog::error!("setgid(): operation not permitted (gid={gid:?}, rgid={rgid:?})");
            *__errno_location() = ErrorCode::OperationNotPermitted.get();
            -1
        },
        Err(error) => {
            ::syslog::error!("setgid(): {error:?} (gid={gid:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
