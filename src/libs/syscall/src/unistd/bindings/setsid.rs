// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new session if the calling process is not a process group leader. In standalone mode
/// this is routed to the process manager daemon, which makes the caller the leader of a brand-new
/// session and process group; in hosted modes it reports the caller as its own session leader.
///
/// # Returns
///
/// Upon successful completion, `setsid()` returns the session ID of the calling process (its own
/// process ID). On failure, it returns `-1` cast to `pid_t` and sets `errno`.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn setsid() -> pid_t {
    #[cfg(feature = "standalone")]
    {
        use crate::errno::__errno_location;

        match ::proc::setsid() {
            Ok(sid) => i32::from(sid),
            Err(e) => {
                ::syslog::warn!("setsid(): failed (error={:?})", e);
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
        use crate::{
            errno::__errno_location,
            unistd,
        };

        match unistd::getpid() {
            Ok(pid) => pid.into(),
            Err(e) => {
                ::syslog::warn!("setsid(): failed (error={:?})", e);
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
