// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd;
use ::sysapi::sys_types::uid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the effective user ID of the calling process. The `geteuid()` function retrieves the
/// effective user ID (EUID) of the calling process, which determines the user permissions used
/// for file access and other user-related operations. The effective user ID is the user ID
/// that the process is currently using for permission checks and may differ from the real user
/// ID if the process has changed its effective user ID through `seteuid()` or similar mechanisms,
/// or if the process is running a setuid program. This function is commonly used by programs that
/// need to determine their current user privileges or by security-conscious applications that need
/// to verify their user identity before performing sensitive operations.
///
/// # Returns
///
/// The `geteuid()` function returns the effective user ID of the calling process on success.
/// The returned value is of type `uid_t` and represents a valid user identifier in the system.
/// On error, it returns `uid_t::MAX` (which corresponds to `-1` cast to `uid_t`) to indicate
/// failure. Unlike most POSIX functions, `geteuid()` traditionally cannot fail and does not
/// modify `errno`. However, in this implementation, errors may occur due to system constraints
/// or internal failures, and such errors are logged but do not set `errno` to maintain POSIX
/// compatibility.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn geteuid() -> uid_t {
    // Get the effective user ID of the calling process and check for errors.
    match unistd::geteuid() {
        // Success.
        Ok(euid) => euid,
        // Failure.
        Err(error) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("geteuid(): failed (error={:?})", error);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // (aka `uid::MAX`) to indicate an error. Hopefully this value does not conflict with a
            // valid user ID.
            uid_t::MAX
        },
    }
}
