// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the current working directory using a file descriptor. The `fchdir()` function changes
/// the current working directory of the calling process to the directory referenced by the file
/// descriptor `fd`. This function is similar to `chdir()` but uses a file descriptor instead of
/// a pathname, which can be more efficient and secure in certain scenarios. The file descriptor
/// must refer to a directory that the calling process has permission to access. This function
/// affects the entire process, and all subsequent relative path operations will be resolved
/// relative to the new working directory. The change persists until the process terminates
/// or another directory change operation is performed.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to a directory that will become the new current working
///   directory. This must be a valid file descriptor that has been opened on a directory with
///   appropriate permissions. The file descriptor should have been obtained from a previous
///   `open()`, `openat()`, or similar system call with the directory specified. The calling
///   process must have search permission for the directory referenced by this file descriptor.
///
/// # Returns
///
/// The `fchdir()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error. Common error conditions include invalid file descriptor, file descriptor
/// does not refer to a directory, permission denied, or the directory is not accessible.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `fd` refers to a valid, open file descriptor.
/// - `fd` refers to a directory (not a regular file or other type of file).
/// - The directory referenced by `fd` has appropriate search permissions for the calling process.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fchdir(fd: c_int) -> c_int {
    // Process system call and check for errors.
    match unistd::fchdir(fd) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("fchdir(): {error:?} (fd={fd:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
