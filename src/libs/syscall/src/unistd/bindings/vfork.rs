// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::fork::fork;
use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new process by duplicating the calling process. Historically `vfork()` differs from
/// `fork()` by suspending the parent and sharing its address space until the child calls `execve()`
/// or `_exit()`. Nanvix provides no such optimization, so `vfork()` is implemented as a plain alias
/// of [`fork()`], which has fully-defined copy-on-write semantics.
///
/// # Returns
///
/// Upon successful completion, `vfork()` returns `0` in the child process and the process identifier
/// of the child process in the parent process. On failure, it returns `-1` in the parent process,
/// no child process is created, and `errno` is set to indicate the error.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn vfork() -> pid_t {
    fork()
}
