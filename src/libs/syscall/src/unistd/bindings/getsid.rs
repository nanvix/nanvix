// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::sys::error::ErrorCode;
use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the session ID of the process identified by `pid`, or of the calling process when `pid`
/// is zero. Nanvix does not implement sessions or process groups, so the calling process is
/// treated as the leader of its own session. Requests for any other process fail because Nanvix
/// does not expose session information for arbitrary process identifiers.
///
/// # Parameters
///
/// - `pid`: The process whose session ID is requested.
///
/// # Returns
///
/// Upon successful completion, `getsid()` returns the session ID of the calling process, which on
/// Nanvix is the process ID of the caller. On failure, it returns `-1` cast to `pid_t`.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getsid(pid: pid_t) -> pid_t {
    if pid < 0 {
        ::syslog::warn!("getsid(pid={:?}): failed (error=InvalidArgument)", pid);
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1 as pid_t;
    }

    match unistd::getpid() {
        Ok(self_pid) => {
            let self_pid: pid_t = self_pid.into();
            if pid == 0 || pid == self_pid {
                self_pid
            } else {
                ::syslog::warn!("getsid(pid={:?}): failed (error=NoSuchProcess)", pid);
                unsafe {
                    *__errno_location() = ErrorCode::NoSuchProcess.get();
                }
                -1 as pid_t
            }
        },
        Err(e) => {
            ::syslog::warn!("getsid(pid={:?}): failed (error={:?})", pid, e);
            // Per POSIX, on failure `-1` is returned and `errno` is set to indicate the error.
            // SAFETY: writing to the thread-local `errno` location is sound.
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1 as pid_t
        },
    }
}
