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
    sys_types::mode_t,
};
use ::syscall::fcntl;

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!(
                "open(): invalid pathname (path={:?}, flags={:?}, mode={:?})",
                path,
                flags,
                mode
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Run system call and check for errors.
    match fcntl::open(pathname, flags, mode) {
        Ok(fd) => fd,
        Err(error) => {
            ::syslog::error!(
                "open(): failed (path={:?}, flags={:?}, mode={:?}, error={:?})",
                pathname,
                flags,
                mode,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}

unsafe extern "C" {
    pub fn fcntl(_fd: c_int, _cmd: c_int, _op: ...);
}

///
/// # Description
///
/// Ensures that the file space is allocated for a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
///
/// # Returns
///
/// Upon success, `posix_fallocate()` empty. Otherwise, it returns an error.
///
/// # Safety
///
/// This function is unsafe because it may access global variables.
///
/// It is safe to call this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_fallocate(fd: c_int, offset: i64, len: i64) -> c_int {
    ::syslog::trace!("posix_fallocate(): fd={:?}, offset={:?}, len={:?}", fd, offset, len);

    // Run system call and check for errors.
    match fcntl::posix_fallocate(fd, offset, len) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "posix_fallocate(): failed (fd={:?}, offset={:?}, len={:?}, error={:?})",
                fd,
                offset,
                len,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Provides advice about the use of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
/// - `advice`: Advice to provide.
///
/// # Returns
///
/// Upon success, `posix_fadvise()` returns zero. Otherwise, it `-1` and sets `errno` to indicate
/// the error.
///
/// # Safety
///
/// This function is unsafe because it may access global variables.
///
/// It is safe to call this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_fadvise(fd: c_int, offset: i64, len: i64, advice: c_int) -> c_int {
    ::syslog::trace!(
        "posix_fadvise(): fd={:?}, offset={:?}, len={:?}, advice={:?}",
        fd,
        offset,
        len,
        advice
    );

    // Run system call and check for errors.
    match fcntl::posix_fadvise(fd, offset, len, advice) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "posix_fadvise(): failed (fd={:?}, offset={:?}, len={:?}, advice={:?}, error={:?})",
                fd,
                offset,
                len,
                advice,
                error
            );
            *__errno_location() = error.code.get();
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn renameat(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
) -> c_int {
    ::syslog::trace!(
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
            ::syslog::error!(
                "renameat(): invalid old pathname (olddirfd={:?}, newdirfd={:?})",
                olddirfd,
                newdirfd
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `newpath` to a Rust string.
    let new_pathname: &str = match ffi::CStr::from_ptr(newpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!(
                "renameat(): invalid new pathname (olddirfd={:?}, newdirfd={:?})",
                olddirfd,
                newdirfd
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Execute system call and check the result.
    match fcntl::renameat(olddirfd, old_pathname, newdirfd, new_pathname) {
        // System call succeeded.
        Ok(()) => 0,
        // System call failed.
        Err(error) => {
            *__errno_location() = error.code.get();
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unlinkat(dirfd: c_int, pathname: *const c_char, flags: c_int) -> c_int {
    ::syslog::trace!("unlinkat(): dirfd={:?}, pathname={:?}, flags={:?}", dirfd, pathname, flags);

    // Attempt to convert `pathname` to a Rust string.
    let path: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("unlinkat(): invalid pathname (dirfd={:?}, flags={:?})", dirfd, flags);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Execute system call and check the result.
    match fcntl::unlinkat(dirfd, path, flags) {
        // System call succeeded.
        Ok(()) => 0,
        // System call failed.
        Err(error) => {
            ::syslog::error!(
                "unlinkat(): failed (dirfd={:?}, pathname={:?}, flags={:?}, error={:?})",
                dirfd,
                path,
                flags,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
