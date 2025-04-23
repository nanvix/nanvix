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
        c_long,
        c_uint,
        c_void,
    },
    sys::types::{
        gid_t,
        mode_t,
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

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn access(_path: *const c_char, _mode: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/355
    ::nvx::error!("access(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn chdir(_path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/358
    ::nvx::error!("chdir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

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
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::unistd::chmod()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn chmod(path: *const c_char, mode: mode_t) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("chmod(): invalid pathname (path={:?}, mode={:?})", path, mode);
            unsafe {
                errno = ErrorCode::InvalidArgument.get();
            }
            return -1;
        },
    };

    match crate::unistd::chmod(path, mode) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("chmod(): failed ({:?})", e);
            errno = e.code.get();
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
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::unistd::chown()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn chown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "chown(): invalid pathname (path={:?}, owner={:?}, group={:?})",
                path,
                owner,
                group
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::unistd::chown(path, owner, group) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("chown(): failed ({:?})", e);
            errno = e.code.get();
            -1
        },
    }
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

/// Dummy implementation of `fchown`.
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn fchown(_fd: i32, _owner: u32, _group: u32) -> isize {
    // TODO:https://github.com/nanvix/nanvix/issues/361
    ::nvx::error!("fchown(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn fdatasync(_fd: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/278
    ::nvx::error!("fdatasync(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
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
/// # See Also
///
/// - [`crate::unistd::ftruncate()`]
///
#[no_mangle]
pub extern "C" fn ftruncate(fd: c_int, length: off_t) -> c_int {
    match crate::unistd::ftruncate(fd, length) {
        Ok(_) => 0,
        Err(e) => {
            unsafe {
                errno = e.code.get();
            }
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
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("lchmod(): invalid pathname (path={:?}, mode={:?})", path, mode);
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::unistd::lchmod(path, mode) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("lchmod(): failed ({:?})", e);
            errno = e.code.get();
            -1
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
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::unistd::lchown()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn lchown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "lchown(): invalid pathname (path={:?}, owner={:?}, group={:?})",
                path,
                owner,
                group
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::unistd::lchown(path, owner, group) {
        Ok(_) => 0,
        Err(e) => {
            unsafe {
                ::nvx::error!("lchown(): failed ({:?})", e);
                errno = e.code.get();
            }
            -1
        },
    }
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
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::link()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn link(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    // Convert C strings to Rust strings.
    let oldpath: &str = match ffi::CStr::from_ptr(oldpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("link(): invalid oldpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };
    let newpath: &str = match ffi::CStr::from_ptr(newpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("link(): invalid newpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let retcode: c_int = crate::unistd::link(oldpath, newpath);

    // Check if the system call failed.
    if retcode < 0 {
        // System call failed. Set errno.
        errno = match ErrorCode::try_from(retcode) {
            Ok(e) => {
                ::nvx::error!("link(): failed ({:?})", e);
                e.get()
            },
            Err(_) => {
                ::nvx::error!("link(): invalid error code ({})", retcode);
                ErrorCode::ValueOutOfRange.get()
            },
        };
        return -1;
    }

    0
}

#[no_mangle]
pub extern "C" fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    ::nvx::trace!("lseek(): fd = {}, offset = {}, whence = {}", fd, offset, whence);
    crate::unistd::lseek(fd, offset, whence)
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
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn read(fd: c_int, buffer: *mut c_void, count: size_t) -> ssize_t {
    ::nvx::trace!("read(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    crate::unistd::read(fd, buffer as *mut u8, count)
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

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn readlink(_path: *const u8, _buf: *mut u8, _bufsize: usize) -> isize {
    // TODO: https://github.com/nanvix/nanvix/issues/531
    ::nvx::error!("readlink(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
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

    let retcode: c_int = crate::unistd::symlink(target, linkpath);

    // Check if the system call failed.
    if retcode < 0 {
        // System call failed. Set errno.
        errno = match ErrorCode::try_from(retcode) {
            Ok(e) => e.get(),
            Err(_) => {
                ::nvx::error!("symlink(): invalid error code ({})", retcode);
                ErrorCode::ValueOutOfRange.get()
            },
        };
        return -1;
    }

    0
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
    crate::unistd::write(fd, buffer as *const u8, count)
}
