// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        gid_t,
        uid_t,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the user and group ownership of a file using a file descriptor. The `fchown()` function
/// changes the ownership of the file referred to by the file descriptor `fd`. This function is
/// similar to `chown()` but operates on an open file descriptor instead of a pathname, which can
/// be more efficient and secure in certain scenarios. The function allows changing the user ID,
/// group ID, or both for the file referenced by the file descriptor. This approach avoids potential
/// race conditions that can occur with pathname-based ownership changes and is particularly useful
/// when the file is already open for other operations.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to the file whose ownership will be changed. This must be a
///   valid file descriptor that has been opened on a file, directory, or other file system object.
///   The file descriptor should have been obtained from a previous `open()`, `openat()`, or similar
///   system call. The calling process must have appropriate permissions to change the ownership of
///   the file referenced by this descriptor.
/// - `owner`: User ID (UID) of the new owner. If this value is `(uid_t)-1`, the user ownership
///   is not changed. The calling process must have appropriate privileges to change ownership
///   to the specified user. Typically, only the superuser or the current owner of the file can
///   change ownership to another user.
/// - `group`: Group ID (GID) of the new group. If this value is `(gid_t)-1`, the group ownership
///   is not changed. The calling process must have appropriate privileges to change ownership
///   to the specified group. Generally, the process must be a member of the target group or
///   have superuser privileges.
///
/// # Returns
///
/// The `fchown()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error. Common error conditions include invalid file descriptor, insufficient
/// privileges to change ownership, invalid user or group ID, or the file system does not support
/// ownership changes.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `fd` refers to a valid, open file descriptor.
/// - The file referenced by `fd` exists and is accessible.
/// - The calling process has appropriate permissions to change ownership of the file.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fchown(fd: c_int, owner: uid_t, group: gid_t) -> c_int {
    // Attempt to change file ownership and check the result.
    match crate::unistd::fchown(fd, owner, group) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("fchown(): {error:?} (fd={fd:?}, owner={owner:?}, group={group:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
