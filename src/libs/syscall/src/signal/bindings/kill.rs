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
/// # Process groups
///
/// In standalone mode the POSIX process-group selectors are honored: a `pid` of `0` signals the
/// caller's process group, `-1` broadcasts to every signalable process, and a `pid` less than `-1`
/// signals the process group whose identifier is `-pid`. These selectors are resolved by the
/// process manager daemon, which owns process-group state. In hosted mode they remain unsupported.
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
            error::Error,
            pm::ProcessIdentifier,
        };

        let target: ProcessIdentifier = ProcessIdentifier::from(pid);

        // A process may post to itself directly in-kernel; cross-process posts and process-group
        // selectors (`pid <= 0`) are routed through the process manager daemon, which enforces the
        // permission policy and fans a group signal out across its members. Both routings converge
        // on the same in-kernel posting primitive.
        let result: Result<(), Error> = if pid > 0 {
            match ::sys::kcall::pm::getpid() {
                Ok(caller) if caller == target => ::sys::kcall::pm::__kcall_kill(caller, signal),
                Ok(_) => ::proc::kill(target, signal),
                Err(e) => Err(e),
            }
        } else {
            // `0`, `-1`, and `< -1` address a process group, whose state the daemon owns; the self
            // fast path never applies, so these always go through the daemon.
            ::proc::kill(target, signal)
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
