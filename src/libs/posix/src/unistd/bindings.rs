// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    fcntl::{
        self,
    },
    ffi::{
        c_char,
        c_int,
        c_long,
        c_uint,
        c_void,
    },
    limits::PATH_MAX,
    sys::types::{
        gid_t,
        off_t,
        pid_t,
        size_t,
        ssize_t,
        uid_t,
    },
    unistd::{
        syscall,
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::core::{
    ffi,
    slice,
};
use ::nvx::sys::error::ErrorCode;

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
#[no_mangle]
pub unsafe extern "C" fn access(path: *const c_char, mode: c_int) -> c_int {
    ::nvx::trace!("access(): path={:?}, mode={:?}", path, mode);
    faccessat(fcntl::AT_FDCWD, path, mode, 0)
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
#[no_mangle]
pub unsafe extern "C" fn chdir(path: *const c_char) -> c_int {
    ::nvx::error!("chdir(): path={:?}", path);

    // Attempt to convert `path`.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("chdir(): invalid path");
            unsafe {
                errno = ErrorCode::InvalidArgument.get();
            }
            return -1;
        },
    };

    // Attempt to change the current working directory and check for errors.
    match crate::unistd::chdir(path) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("chdir(): failed (error={:?})", error);
            unsafe {
                errno = error.code.get();
            }
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
#[no_mangle]
pub unsafe extern "C" fn chown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    ::nvx::trace!("chown(): path={:?}, owner={:?}, group={:?}", path, owner, group);
    fchownat(fcntl::AT_FDCWD, path, owner, group, 0)
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub extern "C" fn chroot(_path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/517
    ::nvx::error!("chroot(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn close(fd: c_int) -> c_int {
    ::nvx::trace!("close(): fd = {}", fd);
    match crate::unistd::close(fd) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("close(): failed ({:?})", error);
            unsafe {
                errno = error.code.get();
            }
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn dup2(_oldfd: c_int, _newfd: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/354
    ::nvx::error!("dup2(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn execve(
    _path: *const c_char,
    _argv: *const *const c_char,
    _envp: *const *const c_char,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/320
    ::nvx::error!("execve(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn _exit(status: c_int) -> ! {
    let Err(e) = nvx::sys::kcall::pm::exit(status);
    panic!("failed to terminate process (error={:?})", e);
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
#[no_mangle]
pub unsafe extern "C" fn faccessat(
    dirfd: c_int,
    path: *const c_char,
    mode: c_int,
    flag: c_int,
) -> c_int {
    ::nvx::trace!(
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
            ::nvx::error!("faccessat(): invalid path");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to check access permissions and check for errors.
    match crate::unistd::faccessat(dirfd, path, mode, flag) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("faccessat(): failed (error={:?})", error);
            errno = error.code.get();
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
#[no_mangle]
pub extern "C" fn fchdir(fd: c_int) -> c_int {
    ::nvx::trace!("fchdir(): fd = {}", fd);

    // Process system call and check for errors.
    match crate::unistd::fchdir(fd) {
        Ok(()) => 0,
        Err(e) => {
            ::nvx::error!("fchdir(): failed ({:?})", e);
            unsafe {
                errno = e.code.get();
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
#[no_mangle]
pub unsafe extern "C" fn fchown(fd: c_int, owner: uid_t, group: gid_t) -> c_int {
    ::nvx::trace!("fchown(): fd={}, owner={}, group={}", fd, owner, group);

    // Attempt to change file ownership and check the result.
    match crate::unistd::fchown(fd, owner, group) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!(
                "fchown(): failed (fd={}, owner={}, group={}, error={:?})",
                fd,
                owner,
                group,
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
#[no_mangle]
pub unsafe extern "C" fn fchownat(
    dirfd: c_int,
    path: *const c_char,
    owner: uid_t,
    group: gid_t,
    flag: c_int,
) -> c_int {
    ::nvx::trace!(
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
            ::nvx::error!(
                "fchownat(): invalid pathname (dirfd={:?}, owner={:?}, group={:?}, flag={:?}, \
                 error={:?})",
                dirfd,
                owner,
                group,
                flag,
                error
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Change file ownership and check the result.
    match crate::unistd::fchownat(dirfd, pathname, owner, group, flag) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!(
                "fchownat(): failed (dirfd={:?}, pathname={:?}, owner={:?}, group={:?}, \
                 flag={:?}, error={:?})",
                dirfd,
                pathname,
                owner,
                group,
                flag,
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
#[no_mangle]
pub unsafe extern "C" fn fdatasync(fd: c_int) -> c_int {
    ::nvx::trace!("fdatasync(): fd={}", fd);

    // Attempt to synchronize the file and check the result.
    match crate::unistd::fdatasync(fd) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("fdatasync(): failed (fd={}, error={:?})", fd, error);
            errno = error.code.get();
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn fork() -> pid_t {
    // TODO: https://github.com/nanvix/nanvix/issues/321
    ::nvx::error!("fork(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
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
#[no_mangle]
pub extern "C" fn fsync(fd: c_int) -> c_int {
    match crate::unistd::fsync(fd) {
        Ok(_) => 0,
        Err(e) => {
            unsafe {
                errno = e.code.get();
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
#[no_mangle]
pub unsafe extern "C" fn ftruncate(fd: c_int, length: off_t) -> c_int {
    ::nvx::trace!("ftruncate(): fd={}, length={}", fd, length);

    // Attempt to truncate the file and check the result.
    match crate::unistd::ftruncate(fd, length) {
        Ok(()) => 0,
        Err(e) => {
            errno = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn getcwd(buf: *mut c_char, size: size_t) -> *mut c_char {
    ::nvx::trace!("getcwd(): buf = {:?}, size = {}", buf, size);

    // Check if the buffer is valid.
    if buf.is_null() {
        ::nvx::error!("getcwd(): invalid buffer");
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
        return core::ptr::null_mut();
    }

    // Get current working directory and check for errors.
    match syscall::getcwd() {
        // Success.
        Ok(cwd) => {
            // Check if the buffer is large enough.
            if cwd.len() + 1 > size as usize {
                ::nvx::error!("getcwd(): buffer is too small");
                unsafe {
                    errno = ErrorCode::ValueOutOfRange.get();
                }
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
            unsafe {
                errno = e.code.get();
            }
            core::ptr::null_mut()
        },
    }
}

///
/// # Safety
///
/// The function has undefined behavior if the `path` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn getentropy(_buffer: *mut c_void, _length: size_t) -> c_int {
    ::nvx::trace!("getentropy(): buffer = {:?}, length = {}", _buffer, _length);

    // Fill buffer with 1s.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(_buffer as *mut u8, _length as usize);
    for byte in buffer.iter_mut() {
        *byte = 1;
    }

    0
}

/// Dummy implementation of `geteuid`.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn geteuid() -> u32 {
    ::nvx::trace!("geteuid(): not implemented, returning 0");
    0
}

#[no_mangle]
pub extern "C" fn getpid() -> pid_t {
    match crate::unistd::getpid() {
        Ok(pid) => pid.into(),
        Err(e) => {
            unsafe {
                errno = e.code.get();
            }
            -1
        },
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn getuid() -> u32 {
    // TODO: https://github.com/nanvix/nanvix/issues/532
    ::nvx::trace!("getuid(): not implemented, returning 0");
    0
}

#[no_mangle]
pub extern "C" fn isatty(_fd: c_int) -> c_int {
    if STDIN_FILENO == _fd || STDOUT_FILENO == _fd || STDERR_FILENO == _fd {
        1
    } else {
        0
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
#[no_mangle]
pub unsafe extern "C" fn lchown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    ::nvx::trace!("lchown(): path={:?}, owner={:?}, group={:?}", path, owner, group);
    fchownat(fcntl::AT_FDCWD, path, owner, group, fcntl::AT_SYMLINK_NOFOLLOW)
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
#[no_mangle]
pub unsafe extern "C" fn link(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    ::nvx::trace!("link(): oldpath={:?}, newpath={:?}", oldpath, newpath);
    linkat(fcntl::AT_FDCWD, oldpath, fcntl::AT_FDCWD, newpath, 0)
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
#[no_mangle]
pub unsafe extern "C" fn linkat(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
    flags: c_int,
) -> c_int {
    ::nvx::trace!(
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
            ::nvx::error!("linkat(): invalid oldpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `newpath`.
    let newpath: &str = match ffi::CStr::from_ptr(newpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("linkat(): invalid newpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Create hard link and parse the result.
    match crate::unistd::linkat(olddirfd, oldpath, newdirfd, newpath, flags) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!(
                "linkat(): failed (olddirfd={}, oldpath={}, newdirfd={}, newpath={}, flags={}, \
                 error={:?})",
                olddirfd,
                oldpath,
                newdirfd,
                newpath,
                flags,
                error
            );
            unsafe {
                errno = error.code.get();
            }
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
#[no_mangle]
pub extern "C" fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    ::nvx::trace!("lseek(): fd={:?}, offset={:?}, whence={:?}", fd, offset, whence);

    // Attempt to seek the file descriptor and check for errors.
    match crate::unistd::lseek(fd, offset, whence) {
        Ok(offset) => offset,
        Err(error) => {
            ::nvx::error!(
                "lseek(): failed (fd={:?}, offset={:?}, whence={:?}, error={:?})",
                fd,
                offset,
                whence,
                error
            );
            unsafe {
                errno = error.code.get();
            }
            -1
        },
    }
}

///
/// # Description
///
/// Creates a pipe.
///
/// # Parameters
///
/// - `fds`: Array to store the file descriptors of the pipe.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
#[no_mangle]
pub extern "C" fn pipe(fds: &mut [c_int; 2]) -> c_int {
    ::nvx::trace!("pipe(): fds = {:?}", fds);

    match crate::unistd::pipe() {
        Ok([read_fd, write_fd]) => {
            fds[0] = read_fd;
            fds[1] = write_fd;
            0
        },
        Err(error) => {
            ::nvx::error!("pipe(): failed (error={:?})", error);
            unsafe {
                errno = error.code.get();
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
#[no_mangle]
pub unsafe extern "C" fn pread(
    fd: c_int,
    buffer: *mut c_void,
    count: size_t,
    offset: off_t,
) -> ssize_t {
    ::nvx::trace!(
        "pread(): fd={}, buffer={:?}, count={:?}, offset={:?}",
        fd,
        buffer,
        count,
        offset
    );

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::nvx::error!(
            "pread(): invalid buffer (fd={:?}, buffer={:?}, count={:?}, offset={:?})",
            fd,
            buffer,
            count,
            offset
        );
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        return 0;
    }

    // Attempt to convert `buffer`.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(buffer as *mut u8, count as usize);

    // Attempt to read from the file descriptor and check for errors.
    match crate::unistd::pread(fd, buffer, offset) {
        Ok(bytes_read) => bytes_read as ssize_t,
        Err(error) => {
            ::nvx::error!(
                "pread(): failed (fd={}, buffer={:?}, count={:?}, offset={:?}, error={:?})",
                fd,
                buffer,
                count,
                offset,
                error
            );
            unsafe {
                errno = error.code.get();
            }
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
#[no_mangle]
pub unsafe extern "C" fn pwrite(
    fd: c_int,
    buffer: *const c_void,
    count: size_t,
    offset: off_t,
) -> ssize_t {
    ::nvx::trace!(
        "pwrite(): fd={}, buffer={:?}, count={:?}, offset={:?}",
        fd,
        buffer,
        count,
        offset
    );

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::nvx::error!(
            "pwrite(): invalid buffer (fd={:?}, buffer={:?}, count={:?}, offset={:?})",
            fd,
            buffer,
            count,
            offset
        );
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // CHeck if count is invalid.
    if count == 0 {
        ::nvx::error!(
            "pwrite(): invalid count (fd={:?}, buffer={:?}, count={:?}, offset={:?})",
            fd,
            buffer,
            count,
            offset
        );
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Attempt to convert `buffer`.
    let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);

    // Attempt to write to the file descriptor and check for errors.
    match crate::unistd::pwrite(fd, buffer, offset) {
        Ok(bytes_written) => bytes_written as ssize_t,
        Err(error) => {
            ::nvx::error!(
                "pwrite(): failed (fd={}, buffer={:?}, count={:?}, offset={:?}, error={:?})",
                fd,
                buffer,
                count,
                offset,
                error
            );
            unsafe {
                errno = error.code.get();
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
#[no_mangle]
pub unsafe extern "C" fn read(fd: c_int, buffer: *mut c_void, count: size_t) -> ssize_t {
    // Skip logging for stdin to avoid spamming the output.
    if fd != STDIN_FILENO {
        ::nvx::trace!("read(): fd={:?}, buffer={:?}, count={:?}", fd, buffer, count);
    }

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::nvx::error!(
            "read(): invalid buffer (fd={:?}, buffer={:?}, count={:?})",
            fd,
            buffer,
            count
        );
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
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
    match crate::unistd::read(fd, buffer) {
        Ok(bytes_read) => bytes_read as ssize_t,
        Err(error) => {
            ::nvx::error!(
                "read(): failed (fd={}, buffer={:?}, count={:?}, error={:?})",
                fd,
                buffer,
                count,
                error
            );
            unsafe {
                errno = error.code.get();
            }
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
#[no_mangle]
pub unsafe extern "C" fn readlinkat(
    dirfd: c_int,
    path: *const c_char,
    buf: *mut c_char,
    bufsize: size_t,
) -> ssize_t {
    ::nvx::trace!(
        "readlinkat(): dirfd={:?}, path={:?}, buf={:?}, bufsize={:?}",
        dirfd,
        path,
        buf,
        bufsize
    );

    // Check if `bufsize` is valid.
    let bufsize: usize = if (bufsize == 0) || (bufsize as usize > PATH_MAX) {
        ::nvx::error!(
            "readlinkat(): invalid buffer size (dirfd={:?}, path={:?}, bufsize={:?})",
            dirfd,
            path,
            bufsize
        );
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
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
            ::nvx::error!(
                "readlinkat(): invalid path (dirfd={:?}, path={:?}, bufsize={:?}, error={:?})",
                dirfd,
                path,
                bufsize,
                error
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Read symbolic link and parse the result.
    match crate::unistd::readlinkat(dirfd, path, buf) {
        Ok(bytes_read) => bytes_read,
        Err(error) => {
            ::nvx::error!(
                "readlinkat(): failed (dirfd={:?}, path={:?}, bufsize={:?}, error={:?})",
                dirfd,
                path,
                bufsize,
                error
            );
            errno = error.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn rmdir(_path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/348
    ::nvx::error!("rmdir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
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
#[no_mangle]
pub unsafe extern "C" fn readlink(
    path: *const c_char,
    buf: *mut c_char,
    bufsize: size_t,
) -> ssize_t {
    ::nvx::trace!("readlink(): path={:?}, buf={:?}, bufsize={:?}", path, buf, bufsize);
    readlinkat(fcntl::AT_FDCWD, path, buf, bufsize)
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub extern "C" fn setgroups(_size: size_t, _list: *const gid_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/523
    ::nvx::error!("setgroups(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
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
#[no_mangle]
pub extern "C" fn sbrk(size: isize) -> *mut u8 {
    match crate::unistd::sbrk(size) {
        // Succeeded to increment the program break.
        Ok(ptr) => ptr,
        // Failed to increment the program break.
        Err(e) => {
            // Set errno.
            unsafe {
                errno = e.code.get();
            }
            (-1_isize) as *mut u8
        },
    }
}

#[no_mangle]
pub extern "C" fn sleep(_seconds: c_uint) -> c_uint {
    // TODO: https://github.com/nanvix/nanvix/issues/453
    ::nvx::error!("sleep(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    0
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
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int {
    // Convert C strings to Rust strings.
    let target: &str = match ffi::CStr::from_ptr(target).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("symlink(): invalid target");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };
    let linkpath: &str = match ffi::CStr::from_ptr(linkpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("symlink(): invalid linkpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Check if the system call failed.
    match crate::unistd::symlink(target, linkpath) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("symlink(): failed (error={:?})", error);
            errno = error.code.get();
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
#[no_mangle]
pub unsafe extern "C" fn symlinkat(
    target: *const c_char,
    dirfd: c_int,
    linkpath: *const c_char,
) -> c_int {
    ::nvx::error!("symlinkat(): target={:?}, dirfd={}, linkpath={:?}", target, dirfd, linkpath);

    // Attempt to convert `target`.
    let target: &str = match ffi::CStr::from_ptr(target).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("symlinkat(): invalid target");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `linkpath`.
    let linkpath: &str = match ffi::CStr::from_ptr(linkpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("symlinkat(): invalid linkpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Create symbolic link and parse the result.
    match crate::unistd::symlinkat(target, dirfd, linkpath) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!(
                "symlinkat(): failed (target={}, dirfd={}, linkpath={}, error={:?})",
                target,
                dirfd,
                linkpath,
                error
            );
            errno = error.code.get();
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn sysconf(_name: c_int) -> c_long {
    // TODO: https://github.com/nanvix/nanvix/issues/342
    ::nvx::error!("sysconf(): not implemented");
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
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn unlink(path: *const c_char) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("unlink(): invalid path");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Process system call and parse result.
    match crate::unistd::unlink(path) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("unlink(): failed (error={:?})", error);
            errno = error.code.get();
            -1
        },
    }
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: size_t) -> ssize_t {
    // Skip logging for stdout and stderr to avoid spamming the output.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        ::nvx::trace!("write(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    }

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::nvx::error!("write(): invalid buffer");
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        ::nvx::error!("write(): invalid write count");
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Construct buffer from raw parts.
    let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);

    // Attempt to write to file descriptor and check for errors.
    match crate::unistd::write(fd, buffer) {
        Ok(bytes_written) => bytes_written as ssize_t,
        Err(error) => {
            ::nvx::error!("write(): failed (error={:?})", error);
            unsafe {
                errno = error.code.get();
            }
            -1
        },
    }
}
