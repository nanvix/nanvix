// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Duplicates a file descriptor. The `dup()` function creates a copy of the file descriptor `fd`
/// using the lowest-numbered available file descriptor. The new file descriptor refers to the same
/// open file description as the original descriptor, and they share file offset and file status
/// flags. This means that if you modify the file offset using one descriptor, it affects the other
/// descriptor as well. The `dup()` function is commonly used for I/O redirection, where a process
/// needs to have multiple file descriptors pointing to the same file or device.
///
/// # Parameters
///
/// - `fd`: File descriptor to duplicate. This must be a valid, open file descriptor. The function
///   will create a new file descriptor that refers to the same open file description as this
///   parameter.
///
/// # Returns
///
/// The `dup()` function returns a new file descriptor that refers to the same open file description
/// as `fd` on success. The new file descriptor is guaranteed to be the lowest-numbered available
/// file descriptor. On error, it returns `-1` and sets `errno` to indicate the error. Common error
/// conditions include invalid file descriptor or no available file descriptors.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `fd` is a valid file descriptor.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dup(fd: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/587
    ::syslog::debug!("dup(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
