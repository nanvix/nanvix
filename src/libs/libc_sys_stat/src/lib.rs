// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

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
    sys_types::{
        dev_t,
        mode_t,
    },
    time::timespec,
};
use ::syscall::{
    errno::__errno_location,
    sys::stat,
};
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
            ::syslog::warn!("fchmod(): {:?} (fd={}, mode={})", error, fd, mode);
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
            ::syslog::warn!(
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
            ::syslog::warn!(
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

/// Resolves a raw `times` pointer into an owned `[timespec; 2]`.
///
/// A NULL pointer maps to `UTIME_NOW` for both entries, per POSIX ("set both
/// timestamps to the current time"). A non-NULL pointer must reference exactly
/// two elements; otherwise [`None`] is returned.
///
/// # Safety
///
/// The caller must ensure a non-NULL `times` points to two valid `timespec`s.
unsafe fn resolve_times(times: *const timespec) -> Option<[timespec; 2]> {
    if times.is_null() {
        let now: timespec = timespec {
            tv_sec: 0,
            tv_nsec: sys_stat::UTIME_NOW,
        };
        return Some([now; 2]);
    }
    slice::from_raw_parts(times, 2).try_into().ok()
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
    let times: [timespec; 2] = match resolve_times(times) {
        Some(times) => times,
        None => {
            ::syslog::warn!("futimens(): invalid times array (fd={})", fd);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to set the access and modification times and parse the result.
    match stat::futimens(fd, &times) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::warn!("futimens(): failed (fd={}, times={:?}, error={:?})", fd, times, error);
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
            ::syslog::warn!("lstat(): invalid pathname");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let statbuf: &mut sys_stat::stat = &mut *statbuf;

    match stat::lstat(pathname, statbuf) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::warn!(
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
            ::syslog::warn!("mkdirat(): invalid pathname");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to create the directory and parse the result.
    match stat::mkdirat(dirfd, pathname, mode) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::warn!(
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

///
/// # Description
///
/// Creates a filesystem node (regular file, device special file, FIFO, or socket) named `path`.
/// Nanvix does not support creating special files through this interface, so the call always fails.
///
/// # Parameters
///
/// - `path`: Pathname of the node to create.
/// - `mode`: File type and permission bits of the new node.
/// - `dev`: Device the new node refers to (used only for device special files).
///
/// # Returns
///
/// The `mknod()` function always returns `-1` and sets `errno` to `ENOTSUP` because Nanvix's
/// filesystem does not support special files.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `path` points to a valid null-terminated C string.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn mknod(path: *const c_char, mode: mode_t, dev: dev_t) -> c_int {
    // Nanvix does not support creating special files; the arguments are unused.
    let _ = (path, mode, dev);
    ::syslog::debug!("mknod(): not supported");
    *__errno_location() = ErrorCode::OperationNotSupported.get();
    -1
}

///
/// # Description
///
/// Creates a new FIFO special file named `path`. Nanvix does not support FIFO special files, so
/// the call always fails.
///
/// # Parameters
///
/// - `path`: Pathname of the FIFO to create.
/// - `mode`: Permission bits of the new FIFO.
///
/// # Returns
///
/// The `mkfifo()` function always returns `-1` and sets `errno` to `ENOTSUP` because Nanvix's
/// filesystem does not support FIFO special files.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `path` points to a valid null-terminated C string.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn mkfifo(path: *const c_char, mode: mode_t) -> c_int {
    // Nanvix does not support FIFO special files; the arguments are unused.
    let _ = (path, mode);
    ::syslog::debug!("mkfifo(): not supported");
    *__errno_location() = ErrorCode::OperationNotSupported.get();
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
pub unsafe extern "C" fn umask(mask: mode_t) -> mode_t {
    match stat::umask(mask) {
        Ok(previous_mask) => previous_mask,
        Err(error) => {
            ::syslog::error!("umask(): failed (mask={:?}, error={:?})", mask, error);
            0
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
            ::syslog::warn!(
                "utimensat(): invalid pathname (dirfd={}, times={:p}, flags={})",
                dirfd,
                times,
                flags
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let times: [timespec; 2] = match resolve_times(times) {
        Some(times) => times,
        None => {
            ::syslog::warn!(
                "utimensat(): invalid times array (dirfd={}, pathname={:?}, flags={})",
                dirfd,
                pathname,
                flags
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match stat::utimensat(dirfd, pathname, &times, flags) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::warn!(
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
