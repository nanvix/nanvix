// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new process by duplicating the calling process. The new (child) process is an exact
/// copy of the calling (parent) process, except for the returned value.
///
/// # Returns
///
/// Upon successful completion, `fork()` returns `0` in the child process and the process identifier
/// of the child process in the parent process. On failure, it returns `-1` in the parent process,
/// no child process is created, and `errno` is set to indicate the error.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn fork() -> pid_t {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
    {
        use crate::errno::__errno_location;

        match crate::unistd::fork::do_fork() {
            Ok(pid) => pid,
            Err(code) => {
                // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
                unsafe {
                    *__errno_location() = code.get();
                }
                -1
            },
        }
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    {
        use crate::errno::__errno_location;
        use ::sys::error::ErrorCode;

        ::syslog::debug!("fork(): not supported on this architecture");
        // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}
