// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
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
/// is zero. This is answered by the process manager daemon, which tracks the session of every
/// process.
///
/// # Parameters
///
/// - `pid`: The process whose session ID is requested.
///
/// # Returns
///
/// On success, the session ID. On failure, `-1` cast to `pid_t` with `errno` set.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getsid(pid: pid_t) -> pid_t {
    if pid < 0 {
        ::syslog::warn!("getsid(pid={:?}): failed (error=InvalidArgument)", pid);
        // SAFETY: writing to the thread-local `errno` location is sound.
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1 as pid_t;
    }

    use ::sys::pm::ProcessIdentifier;

    match ::proc::getsid(ProcessIdentifier::from(pid)) {
        Ok(sid) => i32::from(sid),
        Err(e) => {
            ::syslog::warn!("getsid(pid={:?}): failed (error={:?})", pid, e);
            // SAFETY: writing to the thread-local `errno` location is sound.
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1 as pid_t
        },
    }
}
