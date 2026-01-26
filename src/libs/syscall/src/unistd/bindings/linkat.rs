// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::core::ffi;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new hard link to an existing file relative to directory file descriptors. The
/// `linkat()` function creates a new hard link to an existing file, where both the source and
/// destination paths can be specified relative to directory file descriptors. A hard link is
/// a directory entry that points to the same inode as the original file, effectively creating
/// multiple names for the same file content. Both names refer to the same physical data on disk,
/// and changes made through one name are visible through all other names. This function provides
/// more flexibility than the traditional `link()` function by allowing relative path resolution
/// from specific directory file descriptors, which helps avoid race conditions in pathname-based
/// operations and enables safer file system operations in multithreaded environments.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor used as the starting point for resolving `oldpath`
///   if it is a relative path. If `oldpath` is an absolute path, this parameter is ignored.
///   Use the special value `AT_FDCWD` to specify the current working directory. This must be
///   a valid file descriptor referring to a directory, or `AT_FDCWD`.
/// - `oldpath`: Pathname of the existing file to which the hard link will be created. This must
///   be a valid null-terminated string specifying either an absolute path or a path relative to
///   the directory referenced by `olddirfd`. The file must exist and the calling process must
///   have appropriate permissions to create links to it. The file cannot be a directory unless
///   specific conditions are met and appropriate privileges are held.
/// - `newdirfd`: Directory file descriptor used as the starting point for resolving `newpath`
///   if it is a relative path. If `newpath` is an absolute path, this parameter is ignored.
///   Use the special value `AT_FDCWD` to specify the current working directory. This must be
///   a valid file descriptor referring to a directory, or `AT_FDCWD`.
/// - `newpath`: Pathname for the new hard link to be created. This must be a valid null-terminated
///   string specifying either an absolute path or a path relative to the directory referenced by
///   `newdirfd`. The path must not already exist, and the calling process must have write
///   permission in the directory where the link will be created. The new link will have the
///   same permissions and ownership as the original file.
/// - `flags`: Flags that modify the behavior of the link creation. Common flags include
///   `AT_SYMLINK_FOLLOW` to follow symbolic links when resolving `oldpath`, and `AT_EMPTY_PATH`
///   to allow linking when `oldpath` is an empty string (requiring `olddirfd` to refer to the
///   file directly). The flags provide fine-grained control over link creation behavior and
///   symbolic link handling.
///
/// # Returns
///
/// The `linkat()` function returns `0` on success, indicating that the hard link was created
/// successfully. Both the original file and the new link now refer to the same inode and file
/// content. On error, it returns `-1` and sets `errno` to indicate the specific error condition.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and accesses global state.
/// The function may dereference the `oldpath` and `newpath` pointers to read null-terminated
/// strings, and modifies the global `errno` variable when errors occur.
///
/// It is safe to use this function if the following conditions are met:
/// - The `oldpath` parameter points to a valid null-terminated string or is null only when
///   appropriate flags are set (such as `AT_EMPTY_PATH`).
/// - The `newpath` parameter points to a valid null-terminated string that remains valid for
///   the duration of the function call.
/// - Both directory file descriptors (`olddirfd` and `newdirfd`) are valid open file descriptors
///   referring to directories, or are the special value `AT_FDCWD`.
/// - Access to the global `errno` variable is properly synchronized in multithreaded programs
///   to prevent race conditions during error reporting.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn linkat(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
    flags: c_int,
) -> c_int {
    // Convert `oldpath`.
    let oldpath: &str = {
        // Check if `oldpath` is invalid.
        if oldpath.is_null() {
            ::syslog::error!(
                "linkat(): invalid oldpath (olddirfd={olddirfd:?}, oldpath={oldpath:?}, \
                 newdirfd={newdirfd:?}, newpath={newpath:?}, flags={flags:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Attempt to convert `oldpath`.
        match ffi::CStr::from_ptr(oldpath).to_str() {
            Ok(pathname) => pathname,
            Err(_error) => {
                ::syslog::error!(
                    "linkat(): invalid oldpath (olddirfd={olddirfd:?}, oldpath={oldpath:?}, \
                     newdirfd={newdirfd:?}, newpath={newpath:?}, flags={flags:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        }
    };

    // Convert `newpath`.
    let newpath: &str = {
        // Check if `newpath` is invalid.
        if newpath.is_null() {
            ::syslog::error!(
                "linkat(): invalid newpath (olddirfd={olddirfd:?}, oldpath={oldpath:?}, \
                 newdirfd={newdirfd:?}, newpath={newpath:?}, flags={flags:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Attempt to convert `newpath`.
        match ffi::CStr::from_ptr(newpath).to_str() {
            Ok(pathname) => pathname,
            Err(_error) => {
                ::syslog::error!(
                    "linkat(): invalid newpath (olddirfd={olddirfd:?}, oldpath={oldpath:?}, \
                     newdirfd={newdirfd:?}, newpath={newpath:?}, flags={flags:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        }
    };

    // Create hard link and parse the result.
    match unistd::linkat(olddirfd, oldpath, newdirfd, newpath, flags) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "linkat(): {error:?} (olddirfd={olddirfd:?}, oldpath={oldpath:?}, \
                 newdirfd={newdirfd:?}, newpath={newpath:?}, flags={flags:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
