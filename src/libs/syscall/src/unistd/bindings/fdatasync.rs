// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Synchronizes the data of a file descriptor to persistent storage. The `fdatasync()` function
/// forces all currently queued I/O operations associated with the file descriptor `fd` to the
/// synchronized I/O completion state. This function is similar to `fsync()` but only synchronizes
/// the data portions of a file, not necessarily the metadata. This can be more efficient than
/// `fsync()` when only data integrity is required, as it may not need to update file access times
/// or other metadata that doesn't affect the file's data content. The function ensures that all
/// data written to the file is physically stored on the underlying storage device before returning.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to the file whose data will be synchronized to persistent
///   storage. This must be a valid file descriptor that has been opened for writing or refers
///   to a file that has been modified. The file descriptor should have been obtained from a
///   previous `open()`, `openat()`, or similar system call. The calling process must have
///   appropriate permissions to synchronize the file data.
///
/// # Returns
///
/// The `fdatasync()` function returns `0` on success, indicating that all data has been
/// successfully written to persistent storage. On error, it returns `-1` and sets `errno`
/// to indicate the error. Common error conditions include invalid file descriptor, I/O errors
/// during synchronization, insufficient disk space, or the file descriptor does not support
/// synchronization operations.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `fd` refers to a valid, open file descriptor.
/// - The file referenced by `fd` supports synchronization operations.
/// - The underlying storage device is accessible and functioning properly.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fdatasync(fd: c_int) -> c_int {
    // Attempt to synchronize the file and check the result.
    match crate::unistd::fdatasync(fd) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("fdatasync(): {error:?} (fd={fd:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
