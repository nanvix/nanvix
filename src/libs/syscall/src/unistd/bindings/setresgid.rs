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
/// Sets the real, effective, and saved-set group IDs of the calling process. A value of
/// `(gid_t)-1` leaves the corresponding identifier unchanged.
///
/// Nanvix is a single-user system, so no identifier may actually be changed: the call succeeds
/// only when every requested identifier already equals the current real group ID.
///
/// # Parameters
///
/// - `rgid`: New real group ID, or `(gid_t)-1` to leave it unchanged.
/// - `egid`: New effective group ID, or `(gid_t)-1` to leave it unchanged.
/// - `sgid`: New saved-set group ID, or `(gid_t)-1` to leave it unchanged.
///
/// # Returns
///
/// Upon successful completion, `setresgid()` returns `0`. Otherwise, it returns `-1` and sets
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
pub unsafe extern "C" fn setresgid(rgid: gid_t, egid: gid_t, sgid: gid_t) -> c_int {
    ::syslog::trace!("setresgid(): rgid={rgid:?} egid={egid:?} sgid={sgid:?}");

    match unistd::getgid() {
        Ok(cur) => {
            // A value of (gid_t)-1 means "leave unchanged"; any other value must match the
            // current real group ID because Nanvix cannot switch groups.
            let unchanged = |id: gid_t| id == gid_t::MAX || id == cur;
            if unchanged(rgid) && unchanged(egid) && unchanged(sgid) {
                0
            } else {
                ::syslog::warn!(
                    "setresgid(): operation not permitted (rgid={rgid:?}, egid={egid:?}, \
                     sgid={sgid:?}, cur={cur:?})"
                );
                *__errno_location() = ErrorCode::OperationNotPermitted.get();
                -1
            }
        },
        Err(error) => {
            ::syslog::warn!("setresgid(): {error:?}");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
