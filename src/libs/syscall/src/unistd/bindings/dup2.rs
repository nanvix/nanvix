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
/// Duplicates a file descriptor to a specified file descriptor. The `dup2()` function creates a
/// copy of the file descriptor `oldfd` using the file descriptor number specified by `newfd`. If
/// `newfd` is already open, it is first closed. The new file descriptor refers to the same open
/// file description as the original descriptor, and they share file offset and file status flags.
/// Unlike `dup()`, which uses the lowest available file descriptor, `dup2()` allows you to specify
/// the exact file descriptor number to use. This function is commonly used for I/O redirection in
/// shell implementations and process management.
///
/// # Parameters
///
/// - `oldfd`: File descriptor to duplicate. This must be a valid, open file descriptor. The
///   function will create a new file descriptor that refers to the same open file description
///   as this parameter.
/// - `newfd`: Target file descriptor number. If this file descriptor is already open, it will be
///   closed first. If `newfd` equals `oldfd`, the function returns `newfd` without closing it.
///   The value must be a valid file descriptor number within the system limits.
///
/// # Returns
///
/// The `dup2()` function returns the new file descriptor (`newfd`) on success. On error, it returns
/// `-1` and sets `errno` to indicate the error. Common error conditions include invalid file
/// descriptor, `newfd` out of range, or system resource limitations.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `oldfd` is a valid file descriptor.
/// - `newfd` is within the valid range of file descriptor numbers.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dup2(oldfd: c_int, newfd: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/354
    ::syslog::debug!("dup2(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
