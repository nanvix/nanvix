// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    fcntl,
    ffi::{
        c_char,
        c_int,
    },
    sys::{
        stat,
        types::mode_t,
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
/// Changes the mode of a file.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, `chmod()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `path` points to a valid null-terminated C string.
///
#[no_mangle]
pub unsafe extern "C" fn chmod(path: *const c_char, mode: mode_t) -> c_int {
    ::nvx::trace!("chmod(): path={:?}, mode={}", path, mode);
    fchmodat(fcntl::AT_FDCWD, path, mode, 0)
}

///
/// # Description
///
/// Changes the mode of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, `fchmod()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global state.
///
/// It is safe to call this function if the following conditions are met:
/// - No other thread calls this function at the same time.
///
#[no_mangle]
pub unsafe extern "C" fn fchmod(fd: c_int, mode: mode_t) -> c_int {
    ::nvx::trace!("fchmod(): fd={}, mode={}", fd, mode);

    // Attempt to change the mode and parse the result.
    match crate::sys::stat::fchmod(fd, mode) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("fchmod(): {:?} (fd={}, mode={})", error, fd, mode);
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
/// Upon successful completion, `fchmodat()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `path` points to a valid null-terminated C string.
///
#[no_mangle]
pub unsafe extern "C" fn fchmodat(
    dirfd: c_int,
    path: *const c_char,
    mode: mode_t,
    flag: c_int,
) -> c_int {
    ::nvx::trace!(
        "fchmodat(): dirfd={:?}, path={:?}, mode={:?}, flag={:?}",
        dirfd,
        path,
        mode,
        flag
    );

    // Attempt to convert `path`.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(error) => {
            ::nvx::error!(
                "fchmodat(): invalid pathname (dirfd={:?}, mode={:?}, flag={:?}, error={:?})",
                dirfd,
                mode,
                flag,
                error
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to change the mode and parse the result.
    match crate::sys::stat::fchmodat(dirfd, pathname, mode, flag) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!(
                "fchmodat(): failed (dirfd={}, pathname={:?}, mode={}, flag={}, error={:?})",
                dirfd,
                pathname,
                mode,
                flag,
                error
            );
            errno = error.code.get();
            -1
        },
    }
}

///
/// # Safety
///
/// This function has undefined behavior if buf points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn fstat(fd: c_int, buf: *mut stat::stat) -> c_int {
    ::nvx::trace!("fstat(): fd = {}, buf = {:?}", fd, buf);
    match crate::sys::stat::fstat(fd, &mut *buf) {
        Ok(_) => 0,
        Err(error) => {
            ::nvx::error!("fstat(): failed (fd={}, buf={:p}, error={:?})", fd, buf, error);
            errno = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Changes the mode of a symbolic link.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::unistd::lchmod()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn lchmod(path: *const c_char, mode: mode_t) -> c_int {
    ::nvx::trace!("lchmod(): path={:?}, mode={}", path, mode);
    fchmodat(fcntl::AT_FDCWD, path, mode, fcntl::AT_SYMLINK_NOFOLLOW)
}

///
/// # Description
///
/// Obtains information about the file named `pathname`.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
/// - `statbuf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::sys::stat::lstat`]
///
/// # Safety
///
/// This function has undefined because it dereferences a raw pointer (ie. `statbuf`).
///
#[no_mangle]
pub unsafe extern "C" fn lstat(pathname: *const c_char, statbuf: *mut stat::stat) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("lstat(): invalid pathname");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let statbuf: &mut stat::stat = &mut *statbuf;

    match crate::sys::stat::lstat(pathname, statbuf) {
        Ok(_) => 0,
        Err(error) => {
            ::nvx::error!(
                "lstat(): failed (pathname={}, statbuf={:p}, error={:?})",
                pathname,
                statbuf,
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
/// Obtains information about the file named `pathname`.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
/// - `statbuf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::sys::stat::stat`]
///
/// # Safety
///
/// This function has undefined because it dereferences a raw pointer (ie. `statbuf`).
///
#[no_mangle]
pub unsafe extern "C" fn stat(pathname: *const c_char, statbuf: *mut stat::stat) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("stat(): invalid pathname");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let statbuf: &mut stat::stat = &mut *statbuf;

    match crate::sys::stat::stat(pathname, statbuf) {
        Ok(_) => 0,
        Err(error) => {
            ::nvx::error!(
                "stat(): failed (pathname={}, statbuf={:p}, error={:?})",
                pathname,
                statbuf,
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
/// Creates a new directory.
///
/// # Parameters
///
/// - `pathname`: Pathname of the new directory.
/// - `mode`: Mode of the new directory.
///
/// # Returns
///
/// Upon successful completion, `mkdir()` returns zero. Otherwise, it returns -1 and sets `errno`
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `pathname` points to a valid null-terminated C string.
///
#[no_mangle]
pub unsafe extern "C" fn mkdir(pathname: *const c_char, mode: mode_t) -> c_int {
    ::nvx::trace!("mkdir(): pathname={:?}, mode={}", pathname, mode);
    mkdirat(fcntl::AT_FDCWD, pathname, mode)
}

///
/// # Description
///
/// Creates a new directory relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the new directory.
/// - `mode`: Mode of the new directory.
///
/// # Returns
///
/// Upon successful completion, `mkdirat()` returns zero. Otherwise, it returns -1 and sets `errno`
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `pathname` points to a valid null-terminated C string.
///
#[no_mangle]
pub unsafe extern "C" fn mkdirat(dirfd: c_int, pathname: *const c_char, mode: mode_t) -> c_int {
    ::nvx::trace!("mkdirat(): dirfd={}, pathname={:?}, mode={}", dirfd, pathname, mode);

    // Attempt to convert `pathname`.
    let pathname: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("mkdirat(): invalid pathname");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to create the directory and parse the result.
    match crate::sys::stat::mkdirat(dirfd, pathname, mode) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!(
                "mkdirat(): failed (dirfd={}, pathname={:?}, mode={}, error={:?})",
                dirfd,
                pathname,
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
pub unsafe extern "C" fn truncate(_path: *const c_char, _length: u64) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/454
    ::nvx::error!("truncate(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}
