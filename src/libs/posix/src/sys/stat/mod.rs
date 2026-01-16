// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::{
    ffi,
    slice,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    fcntl::atflags::{
        AT_FDCWD,
        AT_SYMLINK_NOFOLLOW,
    },
    ffi::{
        c_char,
        c_int,
    },
    sys_stat,
    sys_types::mode_t,
    time::timespec,
};
use ::syscall::sys::stat;
use ::syslog::{
    trace_libcall,
    trace_syscall,
};

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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn chmod(path: *const c_char, mode: mode_t) -> c_int {
    fchmodat(AT_FDCWD, path, mode, 0)
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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn fchmod(fd: c_int, mode: mode_t) -> c_int {
    // Attempt to change the mode and parse the result.
    match stat::fchmod(fd, mode) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("fchmod(): {:?} (fd={}, mode={})", error, fd, mode);
            *__errno_location() = error.code.get();
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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn fchmodat(
    dirfd: c_int,
    path: *const c_char,
    mode: mode_t,
    flag: c_int,
) -> c_int {
    // Attempt to convert `path`.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(error) => {
            ::syslog::error!(
                "fchmodat(): invalid pathname (dirfd={:?}, mode={:?}, flag={:?}, error={:?})",
                dirfd,
                mode,
                flag,
                error
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to change the mode and parse the result.
    match stat::fchmodat(dirfd, pathname, mode, flag) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "fchmodat(): failed (dirfd={}, pathname={:?}, mode={}, flag={}, error={:?})",
                dirfd,
                pathname,
                mode,
                flag,
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
/// Sets the access and modification times of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `times`: Access and modification times.
///
/// # Returns
///
/// Upon successful completion, `futimens()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference a raw pointer.
/// - It may modify global state.
///
/// It is safe to call this function if the following conditions are met:
/// - `times` points to an array of `timespec` structures with a length of 2.
/// - This function is not called by multiple threads at the same time.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn futimens(fd: c_int, times: *const timespec) -> c_int {
    // Check if `times` is invalid.
    if times.is_null() {
        ::syslog::error!("futimens(): fd={}, times={:p}", fd, times);
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `times` to a reference to an array of two elements.
    let times: &[timespec; 2] = match slice::from_raw_parts(times, 2).try_into() {
        Ok(array) => array,
        Err(_) => {
            ::syslog::error!("futimens(): invalid times array");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to set the access and modification times and parse the result.
    match stat::futimens(fd, times) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "futimens(): failed (fd={}, times={:?}, error={:?})",
                fd,
                times,
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
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn lchmod(path: *const c_char, mode: mode_t) -> c_int {
    fchmodat(AT_FDCWD, path, mode, AT_SYMLINK_NOFOLLOW)
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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn lstat(pathname: *const c_char, statbuf: *mut sys_stat::stat) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("lstat(): invalid pathname");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let statbuf: &mut sys_stat::stat = &mut *statbuf;

    match stat::lstat(pathname, statbuf) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::error!(
                "lstat(): failed (pathname={}, statbuf={:p}, error={:?})",
                pathname,
                statbuf,
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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn mkdir(pathname: *const c_char, mode: mode_t) -> c_int {
    mkdirat(AT_FDCWD, pathname, mode)
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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn mkdirat(dirfd: c_int, pathname: *const c_char, mode: mode_t) -> c_int {
    // Attempt to convert `pathname`.
    let pathname: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("mkdirat(): invalid pathname");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to create the directory and parse the result.
    match stat::mkdirat(dirfd, pathname, mode) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "mkdirat(): failed (dirfd={}, pathname={:?}, mode={}, error={:?})",
                dirfd,
                pathname,
                mode,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn truncate(_path: *const c_char, _length: u64) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/454
    ::syslog::debug!("truncate(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

///
/// # Description
///
/// Sets the calling process's file mode creation mask (umask).
///
/// # Parameters
///
/// - `mask`: The new file mode creation mask.
///
/// # Returns
///
/// The `umask()` function returns the previous value of the calling process's file mode creation mask.
///
/// # Safety
///
/// This function is safe to call with any valid `mask`.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn umask(mask: u16) -> u16 {
    // TODO: https://github.com/nanvix/nanvix/issues/597.
    ::syslog::debug!("umask(): not implemented");
    0
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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn utimensat(
    dirfd: c_int,
    filename: *const c_char,
    times: *const timespec,
    flags: c_int,
) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(filename).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!(
                "utimensat(): invalid pathname (dirfd={}, times={:p}, flags={})",
                dirfd,
                times,
                flags
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Check if `times` is invalid.
    if times.is_null() {
        ::syslog::error!(
            "utimensat(): invalid times (dirfd={}, pathname={:?}, times={:p}, flags={})",
            dirfd,
            pathname,
            times,
            flags
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `times` to a reference to an array of two elements.
    let times: &[timespec; 2] = match slice::from_raw_parts(times, 2).try_into() {
        Ok(array) => array,
        Err(_) => {
            ::syslog::error!(
                "futimens(): invalid times array (dirfd={}, pathname={:?}, times={:p}, flags={})",
                dirfd,
                pathname,
                times,
                flags
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match stat::utimensat(dirfd, pathname, times, flags) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::error!(
                "utimensat(): failed (dirfd={}, pathname={}, times={:?}, flags={}, error={:?})",
                dirfd,
                pathname,
                times,
                flags,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
