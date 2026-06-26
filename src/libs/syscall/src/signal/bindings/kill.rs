// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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
/// Posts a signal to a process (`kill()`).
///
/// # Parameters
///
/// - `pid`: Process identifier of the target process.
/// - `signal`: Signal number to post, or zero for the null-signal probe.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, `-1` is returned and `errno` is set
/// to indicate the error.
///
/// # Limitations
///
/// Process groups are not supported, so the `pid <= 0` selectors (`-1`, `0`, and `< -1`) are
/// rejected rather than addressing a process group.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub extern "C" fn kill(pid: pid_t, signal: c_int) -> c_int {
    #[cfg(not(feature = "standalone"))]
    {
        use crate::errno::__errno_location;
        use ::sys::error::ErrorCode;

        let _ = (pid, signal);
        ::syslog::debug!("kill(): not supported");
        // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }

    #[cfg(feature = "standalone")]
    {
        use crate::errno::__errno_location;
        use ::sys::{
            error::{
                Error,
                ErrorCode,
            },
            pm::ProcessIdentifier,
        };

        // Process groups are not supported, so only a positive target pid is accepted.
        if pid <= 0 {
            // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return -1;
        }
        let target: ProcessIdentifier = ProcessIdentifier::from(pid);

        // A process may post to itself directly in-kernel; cross-process posts are routed through
        // the process manager daemon, which enforces the permission policy. Both routings converge
        // on the same in-kernel posting primitive.
        let result: Result<(), Error> = match ::sys::kcall::pm::getpid() {
            Ok(caller) if caller == target => ::sys::kcall::pm::__kcall_kill(caller, signal),
            Ok(_) => ::proc::kill(target, signal),
            Err(e) => Err(e),
        };

        match result {
            Ok(()) => 0,
            Err(e) => {
                // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
                unsafe {
                    *__errno_location() = e.code.get();
                }
                -1
            },
        }
    }
}
