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
/// Sets the foreground process group of the terminal referred to by `fd`. Nanvix has a single
/// console shared by one process at a time and no process groups, so this call has no observable
/// side effects and reports success. It exists so that portable software performing job-control
/// bookkeeping compiles, links, and runs.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to the controlling terminal (ignored).
/// - `pgrp`: The process-group ID to make the foreground group.
///
/// # Returns
///
/// `0` on success, or `-1` with `errno` set to `EINVAL` when `pgrp` is negative.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn tcsetpgrp(_fd: c_int, pgrp: pid_t) -> c_int {
    if pgrp < 0 {
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }
    0
}
