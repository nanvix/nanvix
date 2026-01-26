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
    sys_types::uid_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the real user ID of the calling process.
///
/// # Parameters
///
/// - `uid`: New user ID.
///
/// # Returns
///
/// Upon successful completion, `setuid()` returns `0`. Otherwise, it returns `-1` and sets
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
pub unsafe extern "C" fn setuid(uid: uid_t) -> c_int {
    ::syslog::error!("setuid(): uid={uid:?}");

    // Check whether `uid` equals to the real user ID of the calling process.
    match unistd::getuid() {
        Ok(ruid) if uid == ruid => 0,
        Ok(ruid) => {
            ::syslog::error!("setuid(): operation not permitted (uid={uid:?}, ruid={ruid:?})");
            *__errno_location() = ErrorCode::OperationNotPermitted.get();
            -1
        },
        Err(error) => {
            ::syslog::error!("setuid(): {error:?} (uid={uid:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
