// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd;
use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the process ID of the parent of the calling process. The parent relationship is
/// established by the kernel when a process is created and is queried directly through a kernel
/// call.
///
/// # Returns
///
/// Upon successful completion, `getppid()` returns the process ID of the parent of the calling
/// process. On failure, it returns `-1` cast to `pid_t`.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getppid() -> pid_t {
    match unistd::getppid() {
        Ok(parent) => parent.into(),
        Err(e) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("getppid(): failed (error={:?})", e);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // to indicate an error.
            -1 as pid_t
        },
    }
}
