// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi;
use ::sys::error::ErrorCode;
use ::sysapi::{
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
/// Changes the owner and group of a file relative to a directory file descriptor. The `fchownat()`
/// function changes the ownership of the file specified by `path` relative to the directory
/// specified by `dirfd`. This function provides a way to change file ownership using a directory
/// file descriptor as the starting point for relative path resolution, which is useful for avoiding
/// race conditions when working with relative paths. The function can change the user ID, group ID,
/// or both, depending on the values provided. If the file is a symbolic link, the behavior depends
/// on the `flag` parameter.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor that serves as the starting point for relative path
///   resolution. If `path` is absolute, this parameter is ignored. Use `AT_FDCWD` to specify
///   the current working directory.
/// - `path`: Pathname of the file whose ownership will be changed. This can be either an
///   absolute path or a path relative to the directory specified by `dirfd`. Must be a valid
///   null-terminated string.
/// - `owner`: User ID (UID) of the new owner. If this value is `(uid_t)-1`, the user ownership
///   is not changed.
/// - `group`: Group ID (GID) of the new group. If this value is `(gid_t)-1`, the group ownership
///   is not changed.
/// - `flag`: Flags that modify the behavior of the function. Common flags include `AT_SYMLINK_NOFOLLOW`
///   to change the ownership of the symbolic link itself rather than the file it points to.
///
/// # Returns
///
/// The `fchownat()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error. Common error conditions include file not found, permission denied,
/// invalid path, or insufficient privileges to change ownership.
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
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fchownat(
    dirfd: c_int,
    path: *const c_char,
    owner: uid_t,
    group: gid_t,
    flag: c_int,
) -> c_int {
    // Check if `path` is invalid.
    if path.is_null() {
        ::syslog::error!(
            "fchownat(): null path pointer (dirfd={dirfd:?}, path={path:?}, owner={owner:?}, \
             group={group:?}, flag={flag:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `pathname`.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_error) => {
            ::syslog::error!(
                "fchownat(): invalid pathname (dirfd={dirfd:?}, path={path:?}, owner={owner:?}, \
                 group={group:?}, flag={flag:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Change file ownership and check the result.
    match crate::unistd::fchownat(dirfd, pathname, owner, group, flag) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "fchownat(): {error:?} (dirfd={dirfd:?}, path={pathname:?}, owner={owner:?}, \
                 group={group:?}, flag={flag:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
