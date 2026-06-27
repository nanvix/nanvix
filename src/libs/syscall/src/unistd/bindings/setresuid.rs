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
/// Sets the real, effective, and saved-set user IDs of the calling process. A value of
/// `(uid_t)-1` leaves the corresponding identifier unchanged.
///
/// Nanvix is a single-user system, so no identifier may actually be changed: the call succeeds
/// only when every requested identifier already equals the current real user ID.
///
/// # Parameters
///
/// - `ruid`: New real user ID, or `(uid_t)-1` to leave it unchanged.
/// - `euid`: New effective user ID, or `(uid_t)-1` to leave it unchanged.
/// - `suid`: New saved-set user ID, or `(uid_t)-1` to leave it unchanged.
///
/// # Returns
///
/// Upon successful completion, `setresuid()` returns `0`. Otherwise, it returns `-1` and sets
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
pub unsafe extern "C" fn setresuid(ruid: uid_t, euid: uid_t, suid: uid_t) -> c_int {
    ::syslog::trace!("setresuid(): ruid={ruid:?} euid={euid:?} suid={suid:?}");

    match unistd::getuid() {
        Ok(cur) => {
            // A value of (uid_t)-1 means "leave unchanged"; any other value must match the
            // current real user ID because Nanvix cannot switch users.
            let unchanged = |id: uid_t| id == uid_t::MAX || id == cur;
            if unchanged(ruid) && unchanged(euid) && unchanged(suid) {
                0
            } else {
                ::syslog::warn!(
                    "setresuid(): operation not permitted (ruid={ruid:?}, euid={euid:?}, \
                     suid={suid:?}, cur={cur:?})"
                );
                *__errno_location() = ErrorCode::OperationNotPermitted.get();
                -1
            }
        },
        Err(error) => {
            ::syslog::warn!("setresuid(): {error:?}");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
