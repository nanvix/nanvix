// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Truncates a file to a specified length using a file descriptor. The `ftruncate()` function
/// causes the file referred to by the file descriptor `fd` to be truncated to a length of exactly
/// `length` bytes. If the file was previously larger than `length`, the extra data is discarded.
/// If the file was previously shorter than `length`, it is extended with null bytes (`\0`). The
/// file offset is not changed by this operation. This function is similar to `truncate()` but
/// operates on an open file descriptor instead of a pathname, which can be more efficient and
/// secure in certain scenarios as it avoids potential race conditions that can occur with
/// pathname-based operations.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to the file to be truncated. This must be a valid file
///   descriptor that has been opened for writing or with write permissions. The file descriptor
///   should have been obtained from a previous `open()`, `openat()`, or similar system call.
///   The calling process must have appropriate permissions to modify the file referenced by
///   this descriptor.
/// - `length`: The desired new length of the file in bytes. This value must be non-negative.
///   If `length` is greater than the current file size, the file is extended and the extended
///   portion is filled with null bytes. If `length` is less than the current file size, the
///   file is truncated and the data beyond the new length is permanently lost. If `length`
///   equals the current file size, the file remains unchanged.
///
/// # Returns
///
/// The `ftruncate()` function returns `0` on success, indicating that the file has been
/// successfully truncated or extended to the specified length. On error, it returns `-1` and
/// sets `errno` to indicate the error. Common error conditions include invalid file descriptor,
/// insufficient permissions, the file descriptor refers to a non-regular file, insufficient
/// disk space for extension, or the file system does not support truncation operations.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `fd` refers to a valid, open file descriptor with write permissions.
/// - The file referenced by `fd` is a regular file that supports truncation operations.
/// - The file system has sufficient space if the file is being extended.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn ftruncate(fd: c_int, length: off_t) -> c_int {
    // Attempt to truncate the file and check the result.
    match crate::unistd::ftruncate(fd, length) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("ftruncate(): {error:?} (fd={fd:?}, length={length:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
