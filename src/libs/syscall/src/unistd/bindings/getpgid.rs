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
/// Returns the process-group ID of the process identified by `pid`, or of the calling process when
/// `pid` is zero. Nanvix does not implement process groups, so each process is the leader of its
/// own group and the process-group ID equals the process ID. Requests for any other process fail
/// because Nanvix does not expose process-group information for arbitrary process identifiers.
///
/// # Parameters
///
/// - `pid`: The process whose process-group ID is requested.
///
/// # Returns
///
/// On success, the process-group ID (the process ID on Nanvix). On failure, `-1` cast to `pid_t`.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getpgid(pid: pid_t) -> pid_t {
    if pid < 0 {
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
                unsafe {
                    *__errno_location() = ErrorCode::NoSuchProcess.get();
                }
                -1 as pid_t
            }
        },
        Err(e) => {
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1 as pid_t
        },
    }
}
