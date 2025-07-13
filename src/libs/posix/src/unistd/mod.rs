// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;

use ::core::ffi;
use ::sys::error::ErrorCode;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::{
        c_char,
        c_int,
        c_long,
        c_uint,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
        gid_t,
        pid_t,
        uid_t,
    },
};
use ::syscall::unistd::syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmdir(_path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/348
    ::syslog::error!("rmdir(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

///
/// # Description
///
/// Reads the value of a symbolic link.
///
/// # Parameters
///
/// - `path`: Path to the symbolic link.
/// - `buf`: Buffer to store the value of the symbolic link.
/// - `bufsize`: Size of the buffer.
///
/// # Returns
///
/// Upon successful completion, `readlink()` returns the number of bytes read. Otherwise, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - `buf` points to a valid memory location of `bufsize` bytes.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readlink(
    path: *const c_char,
    buf: *mut c_char,
    bufsize: c_size_t,
) -> c_ssize_t {
    ::syslog::trace!("readlink(): path={:?}, buf={:?}, bufsize={:?}", path, buf, bufsize);
    ::syscall::unistd::bindings::readlinkat::readlinkat(AT_FDCWD, path, buf, bufsize)
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub extern "C" fn setgroups(_size: c_size_t, _list: *const gid_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/523
    ::syslog::error!("setgroups(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Increments the program break.
///
/// # Parameters
///
/// - `size`: Number of bytes to increment the program break.
///
/// # Returns
///
/// Upon successful completion, the `sbrk()` function returns the address of the start of the newly
/// allocated memory. Otherwise, it returns `(void *) -1` and sets `errno` to indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::syscall::sbrk()`]
///
#[unsafe(no_mangle)]
pub extern "C" fn sbrk(size: isize) -> *mut u8 {
    match ::syscall::unistd::sbrk(size) {
        // Succeeded to increment the program break.
        Ok(ptr) => ptr,
        // Failed to increment the program break.
        Err(e) => {
            // Set errno.
            unsafe {
                *__errno_location() = e.code.get();
            }
            (-1_isize) as *mut u8
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sleep(_seconds: c_uint) -> c_uint {
    // TODO: https://github.com/nanvix/nanvix/issues/453
    ::syslog::error!("sleep(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    0
}

///
/// # Description
///
/// Sets the effective group ID of the calling process.
///
/// # Parameters
///
/// - `gid`: New group ID.
///
/// # Returns
///
/// Upon successful completion, `setegid()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global variables.
///
/// This function is safe to use if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setegid(gid: gid_t) -> c_int {
    ::syslog::error!("setegid(): gid={:?}", gid);

    // Check wether `gid` equals to the effective group ID of the calling process.
    match syscall::getegid() {
        Ok(egid) if gid == egid => 0,
        Ok(egid) => {
            ::syslog::error!("setegid(): operation not permitted (gid={:?}, egid={:?})", gid, egid);
            *__errno_location() = ErrorCode::OperationNotPermitted.get();
            -1
        },
        Err(error) => {
            ::syslog::error!("setegid(): failed (gid={:?}, error={:?})", gid, error);
            -1
        },
    }
}

///
/// # Description
///
/// Sets the real group ID of the calling process.
///
/// # Parameters
///
/// - `gid`: New group ID.
///
/// # Returns
///
/// Upon successful completion, `setgid()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global variables.
///
/// This function is safe to use if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgid(gid: gid_t) -> c_int {
    ::syslog::error!("setgid(): gid={:?})", gid);

    // Check wether `gid` equals to the real group ID of the calling process.
    match syscall::getgid() {
        Ok(rgid) if gid == rgid => 0,
        Ok(rgid) => {
            ::syslog::error!("setgid(): operation not permitted (gid={:?}, rgid={:?})", gid, rgid);
            *__errno_location() = ErrorCode::OperationNotPermitted.get();
            -1
        },
        Err(error) => {
            ::syslog::error!("setgid(): failed (gid={:?}, error={:?})", gid, error);
            -1
        },
    }
}

///
/// # Description
///
/// Sets the effective user ID of the calling process.
///
/// # Parameters
///
/// - `uid`: New user ID.
///
/// # Returns
///
/// Upon successful completion, `seteuid()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global variables.
///
/// This function is safe to use if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn seteuid(uid: uid_t) -> c_int {
    ::syslog::error!("seteuid(): uid={:?}", uid);

    // Check wether `uid` equals to the effective user ID of the calling process.
    match syscall::geteuid() {
        Ok(euid) if uid == euid => 0,
        Ok(euid) => {
            ::syslog::error!("seteuid(): operation not permitted (uid={:?}, euid={:?})", uid, euid);
            *__errno_location() = ErrorCode::OperationNotPermitted.get();
            -1
        },
        Err(error) => {
            ::syslog::error!("seteuid(): failed (uid={:?}, error={:?})", uid, error);
            -1
        },
    }
}

///
/// # Description
///
/// Sets the real user ID of the calling process.
///
/// # Parameters
///
/// - `uid`: New user ID.
///
/// # Returns
///
/// Upon successful completion, `setuid()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global variables.
///
/// This function is safe to use if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setuid(uid: uid_t) -> c_int {
    ::syslog::error!("setuid(): uid={:?}", uid);

    // Check wether `uid` equals to the real user ID of the calling process.
    match syscall::getuid() {
        Ok(ruid) if uid == ruid => 0,
        Ok(ruid) => {
            ::syslog::error!("setuid(): operation not permitted (uid={:?}, ruid={:?})", uid, ruid);
            *__errno_location() = ErrorCode::OperationNotPermitted.get();
            -1
        },
        Err(error) => {
            ::syslog::error!("setuid(): failed (uid={:?}, error={:?})", uid, error);
            -1
        },
    }
}

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
