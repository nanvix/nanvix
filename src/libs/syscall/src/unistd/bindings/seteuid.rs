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
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the effective user ID of the calling process.
///
/// # Parameters
///
/// - `uid`: New user ID.
///
/// # Returns
///
/// Upon successful completion, `seteuid()` returns `0`. Otherwise, it returns `-1` and sets
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
pub unsafe extern "C" fn seteuid(uid: uid_t) -> c_int {
    ::syslog::error!("seteuid(): uid={uid:?}");

    // Check whether `uid` equals to the effective user ID of the calling process.
    match unistd::geteuid() {
        Ok(euid) if uid == euid => 0,
        Ok(euid) => {
            ::syslog::error!("seteuid(): operation not permitted (uid={uid:?}, euid={euid:?})");
            *__errno_location() = ErrorCode::OperationNotPermitted.get();
            -1
        },
        Err(error) => {
            ::syslog::error!("seteuid(): {error:?} (uid={uid:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
