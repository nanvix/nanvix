// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd;
use ::sysapi::sys_types::gid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the real group ID of the calling process. The `getgid()` function retrieves the
/// real group ID (RGID) of the calling process, which represents the actual group identity of
/// the process as determined when it was created or last modified through appropriate system
/// calls. The real group ID differs from the effective group ID in that it represents the
/// "true" group identity of the process, while the effective group ID determines the group
/// permissions used for access control checks. This function is commonly used by programs that
/// need to determine the original group identity of the process or by applications that need
/// to distinguish between real and effective group identities for security or auditing purposes.
///
/// # Returns
///
/// The `getgid()` function returns the real group ID of the calling process on success.
/// The returned value is of type `gid_t` and represents a valid group identifier in the system.
/// On error, it returns `gid_t::MAX` (which corresponds to `-1` cast to `gid_t`) to indicate
/// failure. Unlike most POSIX functions, `getgid()` traditionally cannot fail and does not
/// modify `errno`. However, in this implementation, errors may occur due to system constraints
/// or internal failures, and such errors are logged but do not set `errno` to maintain POSIX
/// compatibility.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getgid() -> gid_t {
    // Get the real group ID of the calling process and check for errors.
    match unistd::getgid() {
        // Success.
        Ok(gid) => gid,
        // Failure.
        Err(error) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("getgid(): failed (error={:?})", error);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // (aka `gid::MAX`) to indicate an error. Hopefully this value does not conflict with a
            // valid group ID.
            gid_t::MAX
        },
    }
}
