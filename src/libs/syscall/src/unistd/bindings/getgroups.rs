// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
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
/// Returns the supplementary group IDs of the calling process. Nanvix is effectively a single-user
/// system with no supplementary group memberships, so the process always belongs to zero
/// supplementary groups. When `size` is zero the function reports the number of supplementary group
/// IDs without modifying `list`; otherwise the (empty) set is "copied" into `list` and the count is
/// returned.
///
/// # Parameters
///
/// - `size`: The number of elements that `list` can hold.
/// - `list`: Buffer that receives the supplementary group IDs. Left untouched because the set is
///   empty.
///
/// # Returns
///
/// Upon successful completion, the number of supplementary group IDs is returned, which is always
/// `0` on Nanvix. On failure, `-1` is returned and `errno` is set to indicate the error.
///
#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgroups(size: c_int, list: *mut gid_t) -> c_int {
    // A negative buffer size is not a valid argument.
    if size < 0 {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // When the caller requests the group IDs to be stored, the buffer must be valid.
    if size > 0 && list.is_null() {
        *__errno_location() = ErrorCode::BadAddress.get();
        return -1;
    }

    // Nanvix has no supplementary groups, so the result set is always empty regardless of the
    // buffer provided by the caller.
    0
}
