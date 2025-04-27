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
    sys::types::mode_t,
    time::timespec,
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

///
/// # Description
///
/// Unlinks a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the file.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, `unlinkat()` returns zero. Otherwise, it returns -1 and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `pathname` points to a valid null-terminated C string.
///
#[no_mangle]
pub unsafe extern "C" fn unlinkat(dirfd: c_int, pathname: *const c_char, flags: c_int) -> c_int {
    ::nvx::trace!("unlinkat(): dirfd={:?}, pathname={:?}, flags={:?}", dirfd, pathname, flags);

    // Attempt to convert `pathname` to a Rust string.
    let path: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("unlinkat(): invalid pathname (dirfd={:?}, flags={:?})", dirfd, flags);
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Execute system call and check the result.
    match crate::fcntl::unlinkat(dirfd, path, flags) {
        // System call succeeded.
        Ok(()) => 0,
        // System call failed.
        Err(error) => {
            ::nvx::error!(
                "unlinkat(): failed (dirfd={:?}, pathname={:?}, flags={:?}, error={:?})",
                dirfd,
                path,
                flags,
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
/// Sets file access and modification times.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the file.
/// - `times`: Access and modification times.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `filename` points to a valid null-terminated C string.
/// - `times` points to a valid array of length 2 of `timespec` structures.
///
#[no_mangle]
pub unsafe extern "C" fn utimensat(
    dirfd: c_int,
    filename: *const c_char,
    times: *const timespec,
    flags: c_int,
) -> c_int {
    ::nvx::trace!(
        "utimensat(): dirfd={}, filename={:?}, times={:?}, flags={}",
        dirfd,
        filename,
        times,
        flags
    );

    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(filename).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("utimensat(): invalid pathname");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let times: &[timespec; 2] = unsafe { &*(times as *const [timespec; 2]) };

    match crate::fcntl::syscall::utimensat(dirfd, pathname, *times, flags) {
        Ok(_) => 0,
        Err(error) => {
            ::nvx::error!(
                "utimensat(): failed (dirfd={}, pathname={}, times={:?}, flags={}, error={:?})",
                dirfd,
                pathname,
                times,
                flags,
                error
            );
            errno = error.code.get();
            -1
        },
    }
}
