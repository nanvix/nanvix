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
/// Synchronizes a file's in-core state with storage device. The `fsync()` function forces all
/// currently queued I/O operations associated with the file descriptor `fd` to the synchronized
/// I/O completion state. This includes both data and metadata associated with the file. The
/// function ensures that all data written to the file is physically stored on the underlying
/// storage device before returning, providing data integrity guarantees. Unlike `fdatasync()`,
/// which only synchronizes data, `fsync()` also synchronizes file metadata such as access times,
/// modification times, and other file attributes. This comprehensive synchronization makes it
/// suitable for applications that require strong durability guarantees for both file content
/// and metadata.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to the file whose data and metadata will be synchronized
///   to persistent storage. This must be a valid file descriptor that has been opened for writing
///   or refers to a file that has been modified. The file descriptor should have been obtained
///   from a previous `open()`, `openat()`, or similar system call. The calling process must have
///   appropriate permissions to synchronize the file data and metadata.
///
/// # Returns
///
/// The `fsync()` function returns `0` on success, indicating that all data and metadata have
/// been successfully written to persistent storage. On error, it returns `-1` and sets `errno`
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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn fsync(fd: c_int) -> c_int {
    match crate::unistd::fsync(fd) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::error!("fsync(): {error:?} (fd={fd:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
