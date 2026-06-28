// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the process-group ID of the calling process. Nanvix does not implement process groups,
/// so each process is treated as the leader of its own group and the process-group ID equals the
/// process ID. Per POSIX this call always succeeds.
///
/// # Returns
///
/// The process-group ID of the calling process, which on Nanvix is its process ID.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getpgrp() -> pid_t {
    match unistd::getpid() {
        Ok(pid) => pid.into(),
        Err(e) => {
            ::syslog::warn!("getpgrp(): failed (error={:?})", e);
            // Per POSIX, on failure `-1` is returned and `errno` is set to indicate the error.
            // SAFETY: writing to the thread-local `errno` location is sound.
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1 as pid_t
        },
    }
}
