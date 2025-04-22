// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    ffi::{
        c_char,
        c_int,
    },
    sys::types::{
        gid_t,
        mode_t,
        uid_t,
    },
};
use ::core::ffi;
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Opens the file specified by `pathname`.
///
/// # Parameters
///
/// - `path`:  Pathname of the file to open.
/// - `flags`: Flags to open the file.
/// - `mode`:  Mode of the file.
///
/// # Returns
///
/// Upon successful completion, the `open()` system call returns a non-negative integer representing
/// the lowest numbered unused file descriptor. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::fcntl::open()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "open(): invalid pathname (path={:?}, flags={:?}, mode={:?})",
                path,
                flags,
                mode
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Run system call and check for errors.
    match crate::fcntl::open(pathname, flags, mode) {
        Ok(fd) => fd,
        Err(error) => {
            ::nvx::error!(
                "open(): failed (path={:?}, flags={:?}, mode={:?}, error={:?})",
                pathname,
                flags,
                mode,
                error
            );
            errno = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Changes the mode of a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`:  Pathname of the file.
/// - `mode`:  Mode.
/// - `flag`:  Flag.
///
/// # Returns
///
/// Upon successful completion, the `fchmodat()` system call returns `0`. Otherwise, it returns
/// `-1` and sets `errno` to indicate the error.
///
/// # See Also
///
/// - [`crate::fcntl::fchmodat()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn fchmodat(
    dirfd: c_int,
    path: *const c_char,
    mode: mode_t,
    flag: c_int,
) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "fchmodat(): invalid pathname (dirfd={:?}, mode={:?}, flag={:?})",
                dirfd,
                mode,
                flag
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::fcntl::fchmodat(dirfd, pathname, mode, flag) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("fchmodat(): invalid error code");
            errno = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn fchmod(fd: c_int, mode: mode_t) -> c_int {
    match crate::unistd::fchmod(fd, mode) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("fchmod(): invalid error code");
            errno = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn fcntl(_fd: c_int, _cmd: c_int, _op: ...) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/280
    ::nvx::error!(
        "fcntl(): not implemented, ignoring (fd={:?}, cmd={:?}, _op={:?})",
        _fd,
        _cmd,
        _op
    );
    0
}

///
/// # Description
///
/// Changes the owner and group of a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`:  Pathname of the file.
/// - `owner`: Owner of the file.
/// - `group`: Group of the file.
/// - `flag`:  Flag.
///
/// # Returns
///
/// Upon successful completion, the `fchownat()` system call returns `0`. Otherwise, it returns
/// `-1` and sets `errno` to indicate the error.
///
/// # See Also
///
/// - [`crate::fcntl::fchownat()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn fchownat(
    dirfd: c_int,
    path: *const c_char,
    owner: uid_t,
    group: gid_t,
    flag: c_int,
) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "fchownat(): invalid pathname (dirfd={:?}, owner={:?}, group={:?}, flag={:?})",
                dirfd,
                owner,
                group,
                flag
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::fcntl::fchownat(dirfd, pathname, owner, group, flag) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("fchownat(): invalid error code");
            errno = e.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Renames a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor of the old file.
/// - `oldpath`:  Pathname of the old file.
/// - `newdirfd`: Directory file descriptor of the new file.
/// - `newpath`:  Pathname of the new file.
///
/// # Returns
///
/// Upon successful completion, the `renameat()` system call returns `0`. Otherwise, it returns
/// `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `oldpath` points to a valid null-terminated C string.
/// - `newpath` points to a valid null-terminated C string.
///
#[no_mangle]
pub unsafe extern "C" fn renameat(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
) -> c_int {
    ::nvx::trace!(
        "renameat(): olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}",
        olddirfd,
        oldpath,
        newdirfd,
        newpath
    );

    // Attempt to convert `oldpath` to a Rust string.
    let old_pathname: &str = match ffi::CStr::from_ptr(oldpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "renameat(): invalid old pathname (olddirfd={:?}, newdirfd={:?})",
                olddirfd,
                newdirfd
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `newpath` to a Rust string.
    let new_pathname: &str = match ffi::CStr::from_ptr(newpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "renameat(): invalid new pathname (olddirfd={:?}, newdirfd={:?})",
                olddirfd,
                newdirfd
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Execute system call and check the result.
    match crate::fcntl::syscall::renameat(olddirfd, old_pathname, newdirfd, new_pathname) {
        // System call succeeded.
        Ok(()) => 0,
        // System call failed.
        Err(error) => {
            errno = error.code.get();
            -1
        },
    }
}
