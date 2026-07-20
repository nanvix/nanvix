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
/// Returns the process-group ID of the process identified by `pid`, or of the calling process when
/// `pid` is zero. This is answered by the process manager daemon, which tracks the process group of
/// every process.
///
/// # Parameters
///
/// - `pid`: The process whose process-group ID is requested.
///
/// # Returns
///
/// On success, the process-group ID. On failure, `-1` cast to `pid_t` with `errno` set.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getpgid(pid: pid_t) -> pid_t {
    if pid < 0 {
        // SAFETY: writing to the thread-local `errno` location is sound.
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1 as pid_t;
    }

    use ::sys::pm::ProcessIdentifier;

    match ::proc::getpgid(ProcessIdentifier::from(pid)) {
        Ok(pgid) => i32::from(pgid),
        Err(e) => {
            // SAFETY: writing to the thread-local `errno` location is sound.
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1 as pid_t
        },
    }
}
