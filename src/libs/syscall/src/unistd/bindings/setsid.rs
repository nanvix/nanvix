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
/// Creates a new session if the calling process is not a process group leader. Nanvix does not
/// implement sessions or process groups, so this call has no observable side effects; it simply
/// reports the calling process as the leader of a new session by returning its own process ID.
///
/// # Returns
///
/// Upon successful completion, `setsid()` returns the session ID of the calling process, which on
/// Nanvix is the process ID of the caller. On failure, it returns `-1` cast to `pid_t`.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn setsid() -> pid_t {
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
