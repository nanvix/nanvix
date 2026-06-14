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
/// Waits for any child process to change state. This is a convenience wrapper equivalent to
/// `waitpid(-1, stat_loc, 0)`.
///
/// # Parameters
///
/// - `stat_loc`: Pointer to an integer where the exit status of the child process will be stored. A
///   null pointer discards the status.
///
/// # Returns
///
/// Upon successful completion, `wait()` returns the process ID of the child process that changed
/// state. If an error occurs, it returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference `stat_loc`.
///
/// It is safe to use this function if the following conditions are met:
/// - `stat_loc` is either null or points to a valid `c_int`.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn wait(stat_loc: *mut c_int) -> pid_t {
    // SAFETY: The caller guarantees that `stat_loc` is either null or points to a valid `c_int`.
    unsafe { super::waitpid::waitpid(-1, stat_loc, 0) }
}
