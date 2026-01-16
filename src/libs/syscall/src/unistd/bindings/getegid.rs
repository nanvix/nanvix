// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd::{
    self,
};
use ::sysapi::sys_types::gid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the effective group ID of the calling process. The `getegid()` function retrieves the
/// effective group ID (EGID) of the calling process, which determines the group permissions used
/// for file access and other group-related operations. The effective group ID is the group ID
/// that the process is currently using for permission checks and may differ from the real group
/// ID if the process has changed its effective group ID through `setegid()` or similar mechanisms.
/// This function is commonly used by programs that need to determine their current group privileges
/// or by security-conscious applications that need to verify their group identity before performing
/// sensitive operations.
///
///
/// # Returns
///
/// The `getegid()` function returns the effective group ID of the calling process on success.
/// The returned value is of type `gid_t` and represents a valid group identifier in the system.
/// On error, it returns `gid_t::MAX` (which corresponds to `-1` cast to `gid_t`) to indicate
/// failure. Unlike most POSIX functions, `getegid()` traditionally cannot fail and does not
/// modify `errno`. However, in this implementation, errors may occur due to system constraints
/// or internal failures, and such errors are logged but do not set `errno` to maintain POSIX
/// compatibility.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getegid() -> gid_t {
    // Get the effective group ID of the calling process and check for errors.
    match unistd::getegid() {
        // Success.
        Ok(egid) => egid,
        // Failure.
        Err(error) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("getegid(): failed (error={:?})", error);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // (aka `gid::MAX`) to indicate an error. Hopefully this value does not conflict with a
            // valid group ID.
            gid_t::MAX
        },
    }
}
