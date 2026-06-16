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
/// Waits for a process to change state.
///
/// # Parameters
///
/// - `pid`: Process ID of the process to wait for.
/// - `status`: Pointer to an integer where the exit status of the process will be stored.
/// - `options`: Options to control the behavior of the wait operation.
///
/// # Returns
///
/// Upon successful completion, `waitpid()` returns the process ID of the child process that changed
/// state. If `WNOHANG` was specified and no child was ready, `0` is returned. If an error occurs, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Limitations
///
/// The initial implementation reports process *termination* only and carries the following accepted
/// deviations from full POSIX semantics:
/// - Process groups are not supported, so the `pid == 0` and `pid < -1` selectors are treated as
///   "any child of the caller" rather than selecting by process group.
/// - Job control is not supported, so `WUNTRACED` and `WCONTINUED` are accepted for source
///   compatibility but have no effect.
/// - POSIX signals are not supported, so `WIFSIGNALED`/`WTERMSIG` never report a signal death; every
///   reaped child appears as a normal exit.
/// - A blocking wait cannot be interrupted by a signal, so `EINTR` is never returned.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `status` points to a valid `c_int`.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t {
    #[cfg(not(feature = "standalone"))]
    {
        use crate::errno::__errno_location;
        use ::sys::error::ErrorCode;

        let _ = (pid, status, options);
        ::syslog::debug!("waitpid(): not supported");
        // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }

    #[cfg(feature = "standalone")]
    {
        use crate::errno::__errno_location;
        use ::proc::{
            WaitOutcome,
            WaitTarget,
        };
        use ::sys::{
            error::ErrorCode,
            pm::ProcessIdentifier,
        };
        use ::sysapi::sys_wait::{
            WCONTINUED,
            WNOHANG,
            WUNTRACED,
        };

        // Reject unknown option bits.
        if options & !(WNOHANG | WUNTRACED | WCONTINUED) != 0 {
            // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return -1;
        }

        // Map the POSIX `pid` selector onto a wait target. A positive `pid` selects that specific
        // child; `-1`, `0`, and `< -1` all select any child of the caller, because process groups
        // are not supported.
        let target: WaitTarget = if pid > 0 {
            WaitTarget::Pid(ProcessIdentifier::from(pid))
        } else {
            WaitTarget::Any
        };

        match ::proc::wait(target, options) {
            Ok(WaitOutcome::Reaped {
                child,
                status: raw_status,
            }) => {
                // Encode the child's raw exit code into the POSIX wait-status format so that the
                // `WIFEXITED`/`WEXITSTATUS` macros decode it correctly.
                if !status.is_null() {
                    let encoded: c_int = (raw_status & 0xff) << 8;
                    // SAFETY: The caller guarantees that `status` points to a valid `c_int`.
                    unsafe {
                        *status = encoded;
                    }
                }
                i32::from(child)
            },
            Ok(WaitOutcome::NoneReady) => 0,
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
