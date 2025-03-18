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
            errno = ErrorCode::InvalidArgument.into_errno();
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
            errno = error.code.into_errno();
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
            errno = ErrorCode::InvalidArgument.into_errno();
            return -1;
        },
    };

    match crate::fcntl::fchmodat(dirfd, pathname, mode, flag) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("fchmodat(): invalid error code");
            errno = e.code.into_errno();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn fchmod(_fd: c_int, _mode: mode_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/360
    ::nvx::error!("fchmod(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    -1
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn fcntl(_fd: c_int, _cmd: c_int, _op: ...) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/280
    ::nvx::error!("fcntl(): not implemented, ignoring");
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
            errno = ErrorCode::InvalidArgument.into_errno();
            return -1;
        },
    };

    match crate::fcntl::fchownat(dirfd, pathname, owner, group, flag) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("fchownat(): invalid error code");
            errno = e.code.into_errno();
            -1
        },
    }
}
