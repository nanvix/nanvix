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
/// Sets the process-group ID of a process. In standalone mode this is routed to the process manager
/// daemon, which moves the target into the requested process group subject to the POSIX
/// constraints; in hosted modes it validates its arguments and reports success.
///
/// # Parameters
///
/// - `pid`: The process whose process-group ID is to be set (`0` means the calling process).
/// - `pgid`: The target process-group ID (`0` means the process ID of `pid`).
///
/// # Returns
///
/// `0` on success, or `-1` with `errno` set on failure.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn setpgid(pid: pid_t, pgid: pid_t) -> c_int {
    if pid < 0 || pgid < 0 {
        // SAFETY: writing to the thread-local `errno` location is sound.
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    #[cfg(feature = "standalone")]
    {
        use ::sys::pm::ProcessIdentifier;

        match ::proc::setpgid(ProcessIdentifier::from(pid), ProcessIdentifier::from(pgid)) {
            Ok(()) => 0,
            Err(e) => {
                // SAFETY: writing to the thread-local `errno` location is sound.
                unsafe {
                    *__errno_location() = e.code.get();
                }
                -1
            },
        }
    }
    #[cfg(not(feature = "standalone"))]
    {
        0
    }
}
