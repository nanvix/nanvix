// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd;
use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the process ID of the calling process. The `getpid()` function retrieves the process
/// ID (PID) of the calling process, which is a unique identifier assigned to each process in the
/// system. The process ID is used by the operating system to identify and manage processes, and
/// it remains constant for the lifetime of the process. This function is commonly used by programs
/// that need to identify themselves to the system, for logging purposes, or when creating unique
/// identifiers based on the process ID. The returned PID is guaranteed to be unique among all
/// currently running processes in the system.
///
/// # Returns
///
/// The `getpid()` function returns the process ID of the calling process on success. The returned
/// value is of type `pid_t` and represents a valid process identifier in the system. Process IDs
/// are typically positive integers, with PID 1 usually reserved for the init process. On error,
/// it returns `-1` cast to `pid_t` to indicate failure. Unlike most POSIX functions, `getpid()`
/// traditionally cannot fail and does not modify `errno`. However, in this implementation, errors
/// may occur due to system constraints or internal failures, and such errors are logged but do
/// not set `errno` to maintain POSIX compatibility.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getpid() -> pid_t {
    match unistd::getpid() {
        Ok(pid) => pid.into(),
        Err(e) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("getpid(): failed (error={:?})", e);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // to indicate an error. Hopefully this value does not conflict with a valid process ID.
            -1 as pid_t
        },
    }
}
