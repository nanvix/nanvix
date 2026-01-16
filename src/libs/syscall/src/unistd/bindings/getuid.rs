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
/// Returns the real user ID of the calling process. The `getuid()` function retrieves the
/// real user ID (RUID) of the calling process, which represents the actual user identity of
/// the process as determined when it was created or last modified through appropriate system
/// calls. The real user ID differs from the effective user ID in that it represents the
/// "true" user identity of the process, while the effective user ID determines the user
/// permissions used for access control checks. This function is commonly used by programs that
/// need to determine the original user identity of the process or by applications that need
/// to distinguish between real and effective user identities for security or auditing purposes.
///
/// # Returns
///
/// The `getuid()` function returns the real user ID of the calling process on success.
/// The returned value is of type `uid_t` and represents a valid user identifier in the system.
/// On error, it returns `uid_t::MAX` (which corresponds to `-1` cast to `uid_t`) to indicate
/// failure. Unlike most POSIX functions, `getuid()` traditionally cannot fail and does not
/// modify `errno`. However, in this implementation, errors may occur due to system constraints
/// or internal failures, and such errors are logged but do not set `errno` to maintain POSIX
/// compatibility.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getuid() -> uid_t {
    // Get the user ID of the calling process and check for errors.
    match unistd::getuid() {
        // Success.
        Ok(uid) => uid,
        // Failure.
        Err(error) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("getuid(): failed (error={:?})", error);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // (aka `uid::MAX`) to indicate an error. Hopefully this value does not conflict with a
            // valid user ID.
            uid_t::MAX
        },
    }
}
