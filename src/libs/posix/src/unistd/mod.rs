// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;

use ::alloc::{
    ffi::CString,
    string::String,
};
use ::core::{
    ffi,
    slice,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    fcntl::{
        atflags::AT_FDCWD,
        file_access_mode::AT_SYMLINK_NOFOLLOW,
    },
    ffi::{
        c_char,
        c_int,
        c_long,
        c_uint,
        c_void,
    },
    limits::{
        HOST_NAME_MAX,
        PATH_MAX,
    },
    sys_types::{
        c_size_t,
        gid_t,
        off_t,
        pid_t,
        ssize_t,
        uid_t,
    },
    unistd::{
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::syscall::unistd::syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks user's permissions for a file.
///
/// # Parameters
///
/// - `path`: Pathname of the file.
/// - `mode`: Access mode to check.
///
/// # Returns
///
/// Upon successful completion, the `access()` system call returns `0`. Otherwise, it returns `-1`
/// and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because:
/// - It may dereference pointers.
/// - It may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - This function is not called from multiple threads at the same time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn access(path: *const c_char, mode: c_int) -> c_int {
    ::syslog::trace!("access(): path={:?}, mode={:?}", path, mode);
    faccessat(AT_FDCWD, path, mode, 0)
}

///
/// # Description
///
/// Changes the current working directory.
///
/// # Parameters
///
/// - `path`: Pathname of the new working directory.
///
/// # Returns
///
/// Upon successful completion, the `chdir()` system call returns `0`. Otherwise, it returns `-1`
/// and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because:
/// - It may dereference pointers.
/// - It may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chdir(path: *const c_char) -> c_int {
    ::syslog::error!("chdir(): path={:?}", path);

    // Attempt to convert `path`.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("chdir(): invalid path");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to change the current working directory and check for errors.
    match ::syscall::unistd::chdir(path) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("chdir(): failed (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Changes the user and group ownership of a file.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `owner`: User ID of the new owner.
/// - `group`: Group ID of the new owner.
///
/// # Returns
///
/// Upon successful completion, `chown()` returns `0`. Otherwise, it returns `-1` and sets `errno`
/// to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    ::syslog::trace!("chown(): path={:?}, owner={:?}, group={:?}", path, owner, group);
    fchownat(AT_FDCWD, path, owner, group, 0)
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub extern "C" fn chroot(_path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/517
    ::syslog::error!("chroot(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Duplicates a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor to duplicate.
///
/// # Returns
///
/// Upon successful completion, `dup()` returns a new file descriptor that refers to the same open
/// file description as `fd`. Otherwise, it returns `-1` and sets `errno` to indicate the error.
///
#[unsafe(no_mangle)]
pub extern "C" fn dup(fd: c_int) -> c_int {
    ::syslog::trace!("dup(): fd={fd:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/587
    ::syslog::error!("dup(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Duplicates a file descriptor to a specified file descriptor.
///
/// # Parameters
///
/// - `oldfd`: File descriptor to duplicate.
/// - `newfd`: File descriptor to duplicate to.
///
/// # Returns
///
/// Upon successful completion, `dup2()` returns the new file descriptor. Otherwise, it returns
/// `-1` and sets `errno` to indicate the error.
///
#[unsafe(no_mangle)]
pub extern "C" fn dup2(oldfd: c_int, newfd: c_int) -> c_int {
    ::syslog::trace!("dup2(): oldfd={oldfd:?}, newfd={newfd:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/354
    ::syslog::error!("dup2(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Executes a program.
///
/// # Parameters
///
/// - `path`: Path to the executable file.
/// - `argv`: Argument vector.
///
/// # Returns
///
/// Upon successful completion, `execv()` does not return. If it fails, it returns `-1` and sets
/// `errno` to indicate the error.
#[unsafe(no_mangle)]
pub extern "C" fn execv(path: *const c_char, argv: *const *const c_char) -> c_int {
    ::syslog::trace!("execv(): path={path:?}, argv={argv:?}");
    // TODO:https://github.com/nanvix/nanvix/issues/588
    ::syslog::error!("execv(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Executes a program.
///
/// # Parameters
///
/// - `path`: Path to the executable file.
/// - `argv`: Argument vector.
/// - `envp`: Environment variables.
///
/// # Returns
///
/// Upon successful completion, `execve()` does not return. If it fails, it returns `-1` and sets
/// `errno` to indicate the error.
///
#[unsafe(no_mangle)]
pub extern "C" fn execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    ::syslog::trace!("execve(): path={path:?}, argv={argv:?}, envp={envp:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/320
    ::syslog::error!("execve(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Exits the calling process.
///
/// # Parameters
///
/// - `status`: Exit status.
///
/// # Return Values
///
/// This function does not return.
///
#[unsafe(no_mangle)]
pub extern "C" fn _exit(status: c_int) -> ! {
    match sys::kcall::pm::exit(status) {
        Ok(_) => unreachable!("process termination should not successfully return"),
        Err(error) => panic!("failed to terminate process (error={error:?})"),
    }
}

///
/// # Description
///
/// Checks the accessibility of a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`:  Pathname of the file.
/// - `mode`:  Accessibility check mode.
/// - `flag`:  Flag.
///
/// # Returns
///
/// Upon successful completion, `faccessat()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because:
/// - It may dereference pointers.
/// - It may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn faccessat(
    dirfd: c_int,
    path: *const c_char,
    mode: c_int,
    flag: c_int,
) -> c_int {
    ::syslog::trace!(
        "faccessat(): dirfd={:?}, path={:?}, mode={:?}, flag={:?}",
        dirfd,
        path,
        mode,
        flag
    );

    // Attempt to convert `path`.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("faccessat(): invalid path");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to check access permissions and check for errors.
    match ::syscall::unistd::faccessat(dirfd, path, mode, flag) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("faccessat(): failed (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Changes the current working directory.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
#[unsafe(no_mangle)]
pub extern "C" fn fchdir(fd: c_int) -> c_int {
    ::syslog::trace!("fchdir(): fd = {}", fd);

    // Process system call and check for errors.
    match ::syscall::unistd::fchdir(fd) {
        Ok(()) => 0,
        Err(e) => {
            ::syslog::error!("fchdir(): failed ({:?})", e);
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1
        },
    }
}

///
/// # Description
///
/// Changes the owner and group of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `owner`: Owner of the file.
/// - `group`: Group of the file.
///
/// # Returns
///
/// Upon successful completion, `fchown()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fchown(fd: c_int, owner: uid_t, group: gid_t) -> c_int {
    ::syslog::trace!("fchown(): fd={}, owner={}, group={}", fd, owner, group);

    // Attempt to change file ownership and check the result.
    match ::syscall::unistd::fchown(fd, owner, group) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "fchown(): failed (fd={}, owner={}, group={}, error={:?})",
                fd,
                owner,
                group,
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
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fchownat(
    dirfd: c_int,
    path: *const c_char,
    owner: uid_t,
    group: gid_t,
    flag: c_int,
) -> c_int {
    ::syslog::trace!(
        "fchownat(): dirfd={:?}, path={:?}, owner={:?}, group={:?}, flag={:?}",
        dirfd,
        path,
        owner,
        group,
        flag
    );

    // Attempt to convert `pathname`.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(error) => {
            ::syslog::error!(
                "fchownat(): invalid pathname (dirfd={:?}, owner={:?}, group={:?}, flag={:?}, \
                 error={:?})",
                dirfd,
                owner,
                group,
                flag,
                error
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Change file ownership and check the result.
    match ::syscall::unistd::fchownat(dirfd, pathname, owner, group, flag) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "fchownat(): failed (dirfd={:?}, pathname={:?}, owner={:?}, group={:?}, \
                 flag={:?}, error={:?})",
                dirfd,
                pathname,
                owner,
                group,
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
/// Synchronizes the data of a file descriptor to disk.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, `fdatasync()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fdatasync(fd: c_int) -> c_int {
    ::syslog::trace!("fdatasync(): fd={}", fd);

    // Attempt to synchronize the file and check the result.
    match ::syscall::unistd::fdatasync(fd) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("fdatasync(): failed (fd={}, error={:?})", fd, error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fork() -> pid_t {
    // TODO: https://github.com/nanvix/nanvix/issues/321
    ::syslog::error!("fork(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Synchronizes changes to a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::fsync()`]
///
#[unsafe(no_mangle)]
pub extern "C" fn fsync(fd: c_int) -> c_int {
    match ::syscall::unistd::fsync(fd) {
        Ok(_) => 0,
        Err(e) => {
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1
        },
    }
}

///
/// # Description
///
/// Truncates a file to a specified length.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `length`: New size of the file.
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
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ftruncate(fd: c_int, length: off_t) -> c_int {
    ::syslog::trace!("ftruncate(): fd={}, length={}", fd, length);

    // Attempt to truncate the file and check the result.
    match ::syscall::unistd::ftruncate(fd, length) {
        Ok(()) => 0,
        Err(e) => {
            *__errno_location() = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getcwd(buf: *mut c_char, size: c_size_t) -> *mut c_char {
    ::syslog::trace!("getcwd(): buf = {:?}, size = {}", buf, size);

    // Check if the buffer is valid.
    if buf.is_null() {
        ::syslog::error!("getcwd(): invalid buffer");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return core::ptr::null_mut();
    }

    // Get current working directory and check for errors.
    match syscall::getcwd() {
        // Success.
        Ok(cwd) => {
            // Check if the buffer is large enough.
            if cwd.len() + 1 > size as usize {
                ::syslog::error!("getcwd(): buffer is too small");
                *__errno_location() = ErrorCode::ValueOutOfRange.get();
                return core::ptr::null_mut();
            }

            // Copy current working directory to the buffer.
            let cwd: &[u8] = cwd.as_bytes();
            let buf: &mut [u8] = slice::from_raw_parts_mut(buf as *mut u8, size as usize);
            buf[..cwd.len()].copy_from_slice(cwd);

            // Add null terminator.
            buf[cwd.len()] = 0;

            // Return the buffer.
            buf.as_mut_ptr() as *mut c_char
        },
        // Failure.
        Err(e) => {
            *__errno_location() = e.code.get();
            core::ptr::null_mut()
        },
    }
}

///
/// # Description
///
/// Returns the effective group ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getegid()` returns the effective group ID of the calling process.
/// Otherwise, it returns `-1` casted to `gid_t` to indicate the error.
///
/// # Safety
///
/// This function panics if it fails.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getegid() -> gid_t {
    ::syslog::trace!("getegid()");

    // Get the effective group ID of the calling process and check for errors.
    match ::syscall::unistd::getegid() {
        // Success.
        Ok(egid) => egid,
        // Failure.
        Err(error) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("getegid(): failed (error={:?})", error);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // (aka `gid::MAX`) to indicate an error. Hopefully this value does not conflict with a
            // valid group ID.
            gid_t::MAX
        },
    }
}

///
/// # Description
///
/// Returns the effective user ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `geteuid()` returns the effective user ID of the calling process.
/// Otherwise, it returns `-1` casted to `uid_t` to indicate the error.
///
/// # Safety
///
/// This function does not panic but returns a fallback value on failure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn geteuid() -> uid_t {
    ::syslog::trace!("geteuid()");

    // Get the effective user ID of the calling process and check for errors.
    match ::syscall::unistd::geteuid() {
        // Success.
        Ok(euid) => euid,
        // Failure.
        Err(error) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("geteuid(): failed (error={:?})", error);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // (aka `uid::MAX`) to indicate an error. Hopefully this value does not conflict with a
            // valid user ID.
            uid_t::MAX
        },
    }
}

///
/// # Description
///
/// Returns the real group ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getgid()` returns the real group ID of the calling process.
/// Otherwise, it returns `-1` casted to `gid_t` to indicate the error.
///
/// # Safety
///
/// This function does not panic but returns a fallback value on failure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgid() -> gid_t {
    ::syslog::trace!("getgid()");

    // Get the real group ID of the calling process and check for errors.
    match ::syscall::unistd::getgid() {
        // Success.
        Ok(gid) => gid,
        // Failure.
        Err(error) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("getgid(): failed (error={:?})", error);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // (aka `gid::MAX`) to indicate an error. Hopefully this value does not conflict with a
            // valid group ID.
            gid_t::MAX
        },
    }
}

///
/// # Safety
///
/// The function has undefined behavior if the `path` points to an invalid memory location.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getentropy(_buffer: *mut c_void, _length: c_size_t) -> c_int {
    ::syslog::trace!("getentropy(): buffer = {:?}, length = {}", _buffer, _length);

    // Fill buffer with 1s.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(_buffer as *mut u8, _length as usize);
    for byte in buffer.iter_mut() {
        *byte = 1;
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn getpid() -> pid_t {
    match ::syscall::unistd::getpid() {
        Ok(pid) => pid.into(),
        Err(e) => {
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1
        },
    }
}

///
/// # Description
///
/// Returns the user ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getuid()` returns the user ID of the calling process.
/// Otherwise, it returns `-1` casted to `uid_t` to indicate the error.
///
/// # Safety
///
/// This function does not panic but returns a fallback value on failure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getuid() -> uid_t {
    ::syslog::trace!("getuid()");

    // Get the user ID of the calling process and check for errors.
    match ::syscall::unistd::getuid() {
        // Success.
        Ok(uid) => uid,
        // Failure.
        Err(error) => {
            // POSIX does not allow us to modify `errno`. So we just emit a warning.
            ::syslog::warn!("getuid(): failed (error={:?})", error);
            // POSIX does not reserve specific values for errors. We workaround it and return `-1`
            // (aka `uid::MAX`) to indicate an error. Hopefully this value does not conflict with a
            // valid user ID.
            uid_t::MAX
        },
    }
}

///
/// # Description
///
/// Gets the name of the current host.
///
/// # Parameters
///
/// - `name`: Storage location for the host name.
/// - `namelen:  The size of the array pointed to by `name`.
///
/// # Returns
///
/// Upon successful completion, `gethostname()` returns `0`. Otherwise, it returns `-1`.
///
/// # Safety
///
/// This function is unsafe becase it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `name` points to a valid null-terminated string.
/// - `namelen` is a valid size.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostname(name: *mut c_char, namelen: c_size_t) -> c_int {
    ::syslog::trace!("gethostname(): name={:?}, namelen={}", name, namelen);

    // Check if the buffer is valid.
    if name.is_null() {
        ::syslog::error!("gethostname(): invalid buffer (name={:?}, namelen={:?})", name, namelen);
        return -1;
    }

    // Check if `namelen` is invalid.
    if namelen == 0 {
        ::syslog::error!(
            "gethostname(): invalid buffer size (name={:?}, namelen={:?})",
            name,
            namelen
        );
        return -1;
    }

    // Get the host name.
    let hostname: String = syscall::gethostname();

    // Attempt to convert Rust string to C string and check for errors.
    let c_string: CString = match CString::new(hostname) {
        // Success.
        Ok(s) => s,
        // Failure.
        Err(error) => {
            ::syslog::error!(
                "gethostname(): failed to convert string (name={:?}, namelen={:?}, error={:?})",
                name,
                namelen,
                error
            );
            return -1;
        },
    };

    // Check if the buffer is large enough.
    if c_string.as_bytes_with_nul().len() > namelen as usize {
        ::syslog::error!(
            "gethostname(): buffer is too small (name={:?}, namelen={:?})",
            name,
            namelen
        );
        return -1;
    }
    // Truncate the host name to HOST_NAME_MAX if necessary.
    let mut bytes: &[u8] = c_string.as_bytes_with_nul();
    if bytes.len() > HOST_NAME_MAX {
        ::syslog::warn!(
            "gethostname(): hostname is too long, truncating (name={:?}, namelen={:?})",
            name,
            namelen
        );
        bytes = &bytes[..HOST_NAME_MAX];
    }

    // Copy the host name to the buffer.
    let buf: &mut [u8] = slice::from_raw_parts_mut(name as *mut u8, namelen as usize);
    buf[..bytes.len()].copy_from_slice(bytes);

    0
}

///
/// # Description
///
/// Checks if a file descriptor refers to a terminal.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, `isatty()` returns `1` if the file descriptor refers to a terminal.
/// Otherwise, it returns `0` and may set `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn isatty(fd: c_int) -> c_int {
    ::syslog::trace!("isatty(): fd={}", fd);

    match ::syscall::unistd::isatty(fd) {
        Ok(true) => 1,
        Ok(false) => {
            ::syslog::warn!("isatty(): file descriptor is not a terminal (fd={})", fd);
            *__errno_location() = ErrorCode::InvalidTerminalOperation.get();
            0
        },
        Err(error) => {
            ::syslog::error!("isatty(): failed (fd={}, error={:?})", fd, error);
            *__errno_location() = error.code.get();
            0
        },
    }
}

///
/// # Description
///
/// Changes the user and group ownership of a symbolic link.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `owner`: User ID of the new owner.
/// - `group`: Group ID of the new owner.
///
/// # Returns
///
/// Upon successful completion, `lchown()` returns `0`. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lchown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    ::syslog::trace!("lchown(): path={:?}, owner={:?}, group={:?}", path, owner, group);
    fchownat(AT_FDCWD, path, owner, group, AT_SYMLINK_NOFOLLOW)
}

///
/// # Description
///
/// Creates a new hard link to an existing file.
///
/// # Parameters
///
/// - `oldpath`: Path to the file to be linked.
/// - `newpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `link()` returns zero. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// The function is unsafe becase it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `oldpath` points to a valid null-terminated string.
/// - `newpath` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn link(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    ::syslog::trace!("link(): oldpath={:?}, newpath={:?}", oldpath, newpath);
    linkat(AT_FDCWD, oldpath, AT_FDCWD, newpath, 0)
}

///
/// # Description
///
/// Creates a new hard link to an existing file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor of the existing file.
/// - `oldpath`: Path to the existing file.
/// - `newdirfd`: Directory file descriptor of the new file.
/// - `newpath`: Path to the new file.
/// - `flags`: Flags to control the behavior of the system call.
///
/// # Returns
///
/// Upon successful completion, `linkat()` returns zero. Otherwise, it returns -1 and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe becase it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `oldpath` points to a valid null-terminated string.
/// - `newpath` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn linkat(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
    flags: c_int,
) -> c_int {
    ::syslog::trace!(
        "linkat(): olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}, flags={:?}",
        olddirfd,
        oldpath,
        newdirfd,
        newpath,
        flags
    );

    // Attempt to convert `oldpath`.
    let oldpath: &str = match ffi::CStr::from_ptr(oldpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("linkat(): invalid oldpath");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `newpath`.
    let newpath: &str = match ffi::CStr::from_ptr(newpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("linkat(): invalid newpath");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Create hard link and parse the result.
    match ::syscall::unistd::linkat(olddirfd, oldpath, newdirfd, newpath, flags) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "linkat(): failed (olddirfd={}, oldpath={}, newdirfd={}, newpath={}, flags={}, \
                 error={:?})",
                olddirfd,
                oldpath,
                newdirfd,
                newpath,
                flags,
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
/// Sets the file offset of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset to set.
/// - `whence`: Reference point for the offset.
///
/// # Returns
///
/// Upon successful completion, `lseek()` returns the resulting offset. Otherwise, it returns
/// `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub extern "C" fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    ::syslog::trace!("lseek(): fd={:?}, offset={:?}, whence={:?}", fd, offset, whence);

    // Attempt to seek the file descriptor and check for errors.
    match ::syscall::unistd::lseek(fd, offset, whence) {
        Ok(offset) => offset,
        Err(error) => {
            ::syslog::error!(
                "lseek(): failed (fd={:?}, offset={:?}, whence={:?}, error={:?})",
                fd,
                offset,
                whence,
                error
            );
            unsafe {
                *__errno_location() = error.code.get();
            }
            -1
        },
    }
}

///
/// # Description
///
/// Reads data from a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to read into.
/// - `count`: Number of bytes to read.
/// - `offset`: Offset to read from.
///
/// # Returns
///
/// Upon successful completion, `pread()` returns the number of bytes read. Otherwise, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because:
/// - It may dereference pointers.
/// - It may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - `buffer` points to a buffer of `count` bytes.
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pread(
    fd: c_int,
    buffer: *mut c_void,
    count: c_size_t,
    offset: off_t,
) -> ssize_t {
    ::syslog::trace!(
        "pread(): fd={}, buffer={:?}, count={:?}, offset={:?}",
        fd,
        buffer,
        count,
        offset
    );

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!(
            "pread(): invalid buffer (fd={:?}, buffer={:?}, count={:?}, offset={:?})",
            fd,
            buffer,
            count,
            offset
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        return 0;
    }

    // Attempt to convert `buffer`.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(buffer as *mut u8, count as usize);

    // Attempt to read from the file descriptor and check for errors.
    match ::syscall::unistd::pread(fd, buffer, offset) {
        Ok(bytes_read) => bytes_read as ssize_t,
        Err(error) => {
            ::syslog::error!(
                "pread(): failed (fd={}, buffer={:?}, count={:?}, offset={:?}, error={:?})",
                fd,
                buffer,
                count,
                offset,
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
/// Writes data to a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to write.
/// - `count`: Number of bytes to write.
/// - `offset`: Offset to write to.
///
/// # Returns
///
/// Upon successful completion, `pwrite()` returns the number of bytes written. Otherwise, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because:
/// - It may dereference pointers.
/// - It may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - `buffer` points to a valid memory location.
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pwrite(
    fd: c_int,
    buffer: *const c_void,
    count: c_size_t,
    offset: off_t,
) -> ssize_t {
    ::syslog::trace!(
        "pwrite(): fd={}, buffer={:?}, count={:?}, offset={:?}",
        fd,
        buffer,
        count,
        offset
    );

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!(
            "pwrite(): invalid buffer (fd={:?}, buffer={:?}, count={:?}, offset={:?})",
            fd,
            buffer,
            count,
            offset
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // CHeck if count is invalid.
    if count == 0 {
        ::syslog::error!(
            "pwrite(): invalid count (fd={:?}, buffer={:?}, count={:?}, offset={:?})",
            fd,
            buffer,
            count,
            offset
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `buffer`.
    let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);

    // Attempt to write to the file descriptor and check for errors.
    match ::syscall::unistd::pwrite(fd, buffer, offset) {
        Ok(bytes_written) => bytes_written as ssize_t,
        Err(error) => {
            ::syslog::error!(
                "pwrite(): failed (fd={}, buffer={:?}, count={:?}, offset={:?}, error={:?})",
                fd,
                buffer,
                count,
                offset,
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
/// Reads data from a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to read into.
/// - `count`: Number of bytes to read.
///
/// # Returns
///
/// Upon successful completion, `read()` returns the number of bytes read. Otherwise, it returns
/// `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because:
/// - It may dereference pointers.
/// - It may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - `buffer` points to a buffer of `count` bytes.
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn read(fd: c_int, buffer: *mut c_void, count: c_size_t) -> ssize_t {
    // Skip logging for stdin to avoid spamming the output.
    if fd != STDIN_FILENO {
        ::syslog::trace!("read(): fd={:?}, buffer={:?}, count={:?}", fd, buffer, count);
    }

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!(
            "read(): invalid buffer (fd={:?}, buffer={:?}, count={:?})",
            fd,
            buffer,
            count
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        return 0;
    }

    // Construct buffer from raw parts.
    let buffer: &mut [u8] =
        unsafe { ::core::slice::from_raw_parts_mut(buffer as *mut u8, count as usize) };

    // Attempt to read from the file descriptor and check for errors.
    match ::syscall::unistd::read(fd, buffer) {
        Ok(bytes_read) => bytes_read as ssize_t,
        Err(error) => {
            ::syslog::error!(
                "read(): failed (fd={}, buffer={:?}, count={:?}, error={:?})",
                fd,
                buffer,
                count,
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
/// Reads the value of a symbolic link relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`: Path to the symbolic link.
/// - `buf`: Buffer to store the value of the symbolic link.
/// - `bufsize`: Size of the buffer.
///
/// # Returns
///
/// Upon successful completion, `readlinkat()` returns the number of bytes read. Otherwise, it
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
pub unsafe extern "C" fn readlinkat(
    dirfd: c_int,
    path: *const c_char,
    buf: *mut c_char,
    bufsize: c_size_t,
) -> ssize_t {
    ::syslog::trace!(
        "readlinkat(): dirfd={:?}, path={:?}, buf={:?}, bufsize={:?}",
        dirfd,
        path,
        buf,
        bufsize
    );

    // Check if `bufsize` is valid.
    let bufsize: usize = if (bufsize == 0) || (bufsize as usize > PATH_MAX) {
        ::syslog::error!(
            "readlinkat(): invalid buffer size (dirfd={:?}, path={:?}, bufsize={:?})",
            dirfd,
            path,
            bufsize
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    } else {
        bufsize as usize
    };

    // Attempt to convert `path`.
    let buf: &mut [u8] = slice::from_raw_parts_mut(buf as *mut u8, bufsize);

    // Attempt to convert `path`.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(error) => {
            ::syslog::error!(
                "readlinkat(): invalid path (dirfd={:?}, path={:?}, bufsize={:?}, error={:?})",
                dirfd,
                path,
                bufsize,
                error
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Read symbolic link and parse the result.
    match ::syscall::unistd::readlinkat(dirfd, path, buf) {
        Ok(bytes_read) => bytes_read,
        Err(error) => {
            ::syslog::error!(
                "readlinkat(): failed (dirfd={:?}, path={:?}, bufsize={:?}, error={:?})",
                dirfd,
                path,
                bufsize,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}

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
) -> ssize_t {
    ::syslog::trace!("readlink(): path={:?}, buf={:?}, bufsize={:?}", path, buf, bufsize);
    readlinkat(AT_FDCWD, path, buf, bufsize)
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

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: c_size_t) -> ssize_t {
    // Skip logging for stdout and stderr to avoid spamming the output.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        ::syslog::trace!("write(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    }

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!("write(): invalid buffer");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        ::syslog::error!("write(): invalid write count");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Construct buffer from raw parts.
    let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);

    // Attempt to write to file descriptor and check for errors.
    match ::syscall::unistd::write(fd, buffer) {
        Ok(bytes_written) => bytes_written as ssize_t,
        Err(error) => {
            ::syslog::error!("write(): failed (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}
