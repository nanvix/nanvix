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
        c_long,
    },
    sys_types::pid_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a symbolic link named `linkpath` which contains the string `target`.
///
/// # Parameters
///
/// - `target`: Path to the file to be linked.
/// - `linkpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::syscall::symlink()`]
///
#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int {
    // Convert C strings to Rust strings.
    let target: &str = match ffi::CStr::from_ptr(target).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("symlink(): invalid target");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };
    let linkpath: &str = match ffi::CStr::from_ptr(linkpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("symlink(): invalid linkpath");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Check if the system call failed.
    match ::syscall::unistd::symlink(target, linkpath) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("symlink(): failed (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Creates a symbolic link relative to a directory file descriptor.
///
/// # Parameters
///
/// - `target`: Path to the file to be linked.
/// - `dirfd`: Directory file descriptor.
/// - `linkpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `target` points to a valid null-terminated string.
/// - `linkpath` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symlinkat(
    target: *const c_char,
    dirfd: c_int,
    linkpath: *const c_char,
) -> c_int {
    ::syslog::error!("symlinkat(): target={:?}, dirfd={}, linkpath={:?}", target, dirfd, linkpath);

    // Attempt to convert `target`.
    let target: &str = match ffi::CStr::from_ptr(target).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("symlinkat(): invalid target");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `linkpath`.
    let linkpath: &str = match ffi::CStr::from_ptr(linkpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("symlinkat(): invalid linkpath");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Create symbolic link and parse the result.
    match ::syscall::unistd::symlinkat(target, dirfd, linkpath) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "symlinkat(): failed (target={}, dirfd={}, linkpath={}, error={:?})",
                target,
                dirfd,
                linkpath,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sysconf(_name: c_int) -> c_long {
    // TODO: https://github.com/nanvix/nanvix/issues/342
    ::syslog::error!("sysconf(): not implemented");
    0
}

///
/// # Description
///
/// Deletes a name from the filesystem.
///
/// # Parameters
///
/// - `path`: Path to the file to be unlinked.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::unlink()`]
///
#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn unlink(path: *const c_char) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("unlink(): invalid path");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Process system call and parse result.
    match ::syscall::unistd::unlink(path) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("unlink(): failed (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Waits for a process to change state.
///
/// # Parameters
///
/// - `pid`: Process ID of the process to wait for.
/// - `status`: Pointer to an integer where the exit status of the process will be stored.
/// - `options`: Options to control the behavior of the wait operation.
///
/// # Returns
///
/// Upon successful completion, `waitpid()` returns the process ID of the child process that changed
/// state. If an error occurs, it returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `status` points to a valid `c_int`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t {
    ::syslog::trace!("waitpid(): pid={pid:?}, status={status:?}, options={options:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/336.
    ::syslog::error!("waitpid(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
