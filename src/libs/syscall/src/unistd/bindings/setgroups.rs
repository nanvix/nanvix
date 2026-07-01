// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::core::slice;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        c_size_t,
        gid_t,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the supplementary group IDs of the calling process. Nanvix is a single-user system with no
/// supplementary group memberships, so the only group that may appear in `list` is the calling
/// process's current real group ID; requesting any other group cannot be honored.
///
/// # Parameters
///
/// - `size`: Number of entries in `list`.
/// - `list`: Array of `size` supplementary group IDs.
///
/// # Returns
///
/// Upon successful completion, `setgroups()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `list` points to a valid array of at least `size` `gid_t` values, or `size` is zero.
/// - This function is not called from multiple threads at the same time.
///
#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgroups(size: c_size_t, list: *const gid_t) -> c_int {
    // Clearing the (already empty) supplementary group list always succeeds.
    if size == 0 {
        return 0;
    }

    // A non-empty list must be a valid pointer.
    if list.is_null() {
        ::syslog::warn!("setgroups(): null list pointer (size={size})");
        *__errno_location() = ErrorCode::BadAddress.get();
        return -1;
    }

    match unistd::getgid() {
        Ok(cur) => {
            // Nanvix has no supplementary group memberships, so the only group that can appear in
            // the list is the current real group ID.
            let groups: &[gid_t] = slice::from_raw_parts(list, size as usize);
            if groups.iter().all(|&gid| gid == cur) {
                0
            } else {
                ::syslog::warn!("setgroups(): operation not permitted (size={size}, cur={cur:?})");
                *__errno_location() = ErrorCode::OperationNotPermitted.get();
                -1
            }
        },
        Err(error) => {
            ::syslog::warn!("setgroups(): {error:?}");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
