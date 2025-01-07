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
        c_void,
    },
    sys::types::{
        off_t,
        pid_t,
        size_t,
        ssize_t,
    },
};
use ::core::ffi;
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub extern "C" fn close(fd: c_int) -> c_int {
    ::nvx::log!("close(): fd = {}", fd);
    crate::unistd::close(fd)
}

#[no_mangle]
pub extern "C" fn _exit(status: c_int) -> ! {
    let Err(e) = nvx::sys::kcall::pm::exit(status);
    panic!("failed to terminate process (error={:?})", e);
}

///
/// # Safety
///
/// The function has undefined behavior if the `path` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn getentropy(_buffer: *mut c_void, _length: size_t) -> c_int {
    ::nvx::log!("getentropy(): buffer = {:?}, length = {}", _buffer, _length);
    -1
}

#[no_mangle]
pub extern "C" fn getpid() -> pid_t {
    match nvx::sys::kcall::pm::getpid() {
        Ok(pid) => pid.into(),
        Err(e) => {
            unsafe {
                errno = e.code.into_errno();
            }
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn isatty(_fd: c_int) -> c_int {
    // TODO: Implement this system call.
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    -1
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
#[no_mangle]
pub extern "C" fn link(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    // Convert C strings to Rust strings.
    let oldpath: &str = match unsafe { ffi::CStr::from_ptr(oldpath).to_str() } {
        Ok(pathname) => pathname,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };
    let newpath: &str = match unsafe { ffi::CStr::from_ptr(newpath).to_str() } {
        Ok(pathname) => pathname,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };

    let retcode: c_int = crate::unistd::link(oldpath, newpath);

    // Check if the system call failed.
    if retcode < 0 {
        // System call failed. Set errno.
        unsafe {
            errno = match ErrorCode::try_from(retcode) {
                Ok(e) => e.into_errno(),
                Err(_) => {
                    ::nvx::log!("link(): invalid error code ({})", retcode);
                    ErrorCode::ValueOutOfRange.into_errno()
                },
            }
        }
        return -1;
    }

    0
}

#[no_mangle]
pub extern "C" fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    ::nvx::log!("lseek(): fd = {}, offset = {}, whence = {}", fd, offset, whence);
    crate::unistd::lseek(fd, offset, whence)
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn read(fd: c_int, buffer: *mut c_void, count: size_t) -> ssize_t {
    ::nvx::log!("read(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    crate::unistd::read(fd, buffer as *mut u8, count)
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
                errno = e.code.into_errno();
            }
            (-1_isize) as *mut u8
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
#[no_mangle]
pub extern "C" fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int {
    // Convert C strings to Rust strings.
    let target: &str = match unsafe { ffi::CStr::from_ptr(target).to_str() } {
        Ok(pathname) => pathname,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };
    let linkpath: &str = match unsafe { ffi::CStr::from_ptr(linkpath).to_str() } {
        Ok(pathname) => pathname,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };

    let retcode: c_int = crate::unistd::symlink(target, linkpath);

    // Check if the system call failed.
    if retcode < 0 {
        // System call failed. Set errno.
        unsafe {
            errno = match ErrorCode::try_from(retcode) {
                Ok(e) => e.into_errno(),
                Err(_) => {
                    ::nvx::log!("symlink(): invalid error code ({})", retcode);
                    ErrorCode::ValueOutOfRange.into_errno()
                },
            }
        }
        return -1;
    }

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
pub extern "C" fn unlink(path: *const c_char) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match unsafe { ffi::CStr::from_ptr(path).to_str() } {
        Ok(pathname) => pathname,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };

    let retcode: c_int = crate::unistd::unlink(path);

    // Check if the system call failed.
    if retcode < 0 {
        // System call failed. Set errno.
        unsafe {
            errno = match ErrorCode::try_from(retcode) {
                Ok(e) => e.into_errno(),
                Err(_) => {
                    ::nvx::log!("unlink(): invalid error code ({})", retcode);
                    ErrorCode::ValueOutOfRange.into_errno()
                },
            }
        }
        return -1;
    }

    0
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: size_t) -> ssize_t {
    ::nvx::log!("write(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    crate::unistd::write(fd, buffer as *const u8, count)
}
