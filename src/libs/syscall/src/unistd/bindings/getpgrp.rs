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
/// Returns the process-group ID of the calling process. In standalone mode this is answered by the
/// process manager daemon; in hosted modes the caller is treated as the leader of its own group, so
/// the process-group ID equals the process ID. Per POSIX this call always succeeds.
///
/// # Returns
///
/// The process-group ID of the calling process, or `-1` cast to `pid_t` with `errno` set on the
/// rare failure to determine it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getpgrp() -> pid_t {
    #[cfg(feature = "standalone")]
    {
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
    #[cfg(not(feature = "standalone"))]
    {
        use crate::unistd;

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
}
