// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the process-group ID of a process. Nanvix does not implement process groups, so this call
/// has no observable side effects; it validates its arguments and reports success so that portable
/// software performing job-control bookkeeping compiles, links, and runs.
///
/// # Parameters
///
/// - `pid`: The process whose process-group ID is to be set (`0` means the calling process).
/// - `pgid`: The target process-group ID (`0` means the process ID of `pid`).
///
/// # Returns
///
/// `0` on success, or `-1` with `errno` set to `EINVAL` when an argument is negative.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn setpgid(pid: pid_t, pgid: pid_t) -> c_int {
    if pid < 0 || pgid < 0 {
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }
    0
}
