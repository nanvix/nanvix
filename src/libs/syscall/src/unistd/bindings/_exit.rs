// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Terminates the calling process immediately. The `_exit()` function terminates the calling
/// process without performing any cleanup operations that would normally be performed by `exit()`.
/// This function does not call any functions registered with `atexit()` or `on_exit()`, does not
/// flush stdio streams, and does not remove temporary files. The process terminates immediately
/// and the exit status is made available to the parent process. This function is typically used
/// in child processes after a `fork()` operation when an error occurs and immediate termination
/// is required without affecting the parent process's state. It is also used in signal handlers
/// and other contexts where normal cleanup might be unsafe or undesirable.
///
/// # Parameters
///
/// - `status`: Exit status code to be returned to the parent process. This value is typically
///   used to indicate the success or failure of the process. By convention, a status of `0`
///   indicates successful termination, while non-zero values indicate various error conditions.
///   The exact meaning of non-zero values is application-specific. The status value is made
///   available to the parent process through `wait()` or `waitpid()` system calls.
///
/// # Returns
///
/// This function does not return to the caller. The process is terminated immediately and
/// control never returns to the calling code. The function is marked with the `!` (never)
/// return type to indicate that execution flow ends here. If the underlying system call
/// fails to terminate the process, the function will panic to ensure the process does not
/// continue execution unexpectedly.
///
/// # Safety
///
/// This function is unsafe because it may modify global state and terminate the process.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - The calling process is in a state where immediate termination is safe.
/// - No critical cleanup operations are required that would be skipped by immediate termination.
/// - The process is not holding locks or resources that require explicit cleanup.
/// - The exit status value is meaningful to the parent process or system.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _exit(status: c_int) -> ! {
    match sys::kcall::pm::exit(status) {
        Ok(_) => unreachable!("process termination should not successfully return"),
        Err(error) => panic!("failed to terminate process (error={error:?})"),
    }
}
