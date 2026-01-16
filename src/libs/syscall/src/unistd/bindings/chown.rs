// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::{
        c_char,
        c_int,
    },
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
/// Changes the user and group ownership of a file. The `chown()` function changes the ownership
/// of the file specified by `path`. If the file is a symbolic link, the ownership of the file
/// that the symbolic link refers to is changed, not the symbolic link itself. This is in contrast
/// to `lchown()` which changes the ownership of the symbolic link itself. The function allows
/// changing the user ID, group ID, or both for the specified file. This function is commonly used
/// for file system management and access control operations.
///
/// # Parameters
///
/// - `path`: Pathname of the file whose ownership will be changed. This must be a valid
///   null-terminated string specifying either an absolute or relative path to an existing file
///   or directory. The calling process must have appropriate permissions to change the ownership
///   of the specified file.
/// - `owner`: User ID (UID) of the new owner. If this value is `(uid_t)-1`, the user ownership
///   is not changed. The calling process must have appropriate privileges to change ownership
///   to the specified user.
/// - `group`: Group ID (GID) of the new group. If this value is `(gid_t)-1`, the group ownership
///   is not changed. The calling process must have appropriate privileges to change ownership
///   to the specified group.
///
/// # Returns
///
/// The `chown()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error. Common error conditions include file not found, permission denied,
/// invalid user or group ID, or path is not accessible.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - `path` remains valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn chown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    crate::unistd::bindings::fchownat::fchownat(AT_FDCWD, path, owner, group, 0)
}
