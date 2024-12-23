// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    ffi::{
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
pub unsafe extern "C" fn getentropy(_buffer: *mut u8, _length: u32) -> i32 {
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
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: size_t) -> ssize_t {
    ::nvx::log!("write(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    crate::unistd::write(fd, buffer as *const u8, count)
}
