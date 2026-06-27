// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
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
/// Returns the foreground process-group ID associated with the terminal referred to by `fd`.
/// Nanvix has a single console shared by one process at a time and no process groups, so the
/// calling process is always the foreground process; this returns its process ID.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to the controlling terminal (ignored).
///
/// # Returns
///
/// The foreground process-group ID (the calling process's ID on Nanvix), or `-1` on failure.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn tcgetpgrp(_fd: c_int) -> pid_t {
    match unistd::getpid() {
        Ok(pid) => pid.into(),
        Err(e) => {
            ::syslog::warn!("tcgetpgrp(): failed (error={:?})", e);
            // Per POSIX, on failure `-1` is returned and `errno` is set to indicate the error.
            // SAFETY: writing to the thread-local `errno` location is sound.
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1 as pid_t
        },
    }
}
