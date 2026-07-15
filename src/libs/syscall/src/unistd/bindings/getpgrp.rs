// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the process-group ID of the calling process. This is answered by the process manager
/// daemon. Per POSIX this call always succeeds.
///
/// # Returns
///
/// The process-group ID of the calling process, or `-1` cast to `pid_t` with `errno` set on the
/// rare failure to determine it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getpgrp() -> pid_t {
    match ::proc::getpgrp() {
        Ok(pgid) => i32::from(pgid),
        Err(e) => {
            ::syslog::warn!("getpgrp(): failed (error={:?})", e);
            // SAFETY: writing to the thread-local `errno` location is sound.
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1 as pid_t
        },
    }
}
