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
    },
    sys::stat,
};
use ::core::ffi;
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Safety
///
/// This function has undefined behavior if buf points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn fstat(fd: c_int, buf: *mut stat::stat) -> c_int {
    ::nvx::trace!("fstat(): fd = {}, buf = {:?}", fd, buf);
    crate::sys::stat::fstat(fd, &mut *buf)
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
#[no_mangle]
pub unsafe extern "C" fn lstat(pathname: *const c_char, statbuf: *mut stat::stat) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("lstat(): invalid pathname");
            errno = ErrorCode::InvalidArgument.into_errno();
            return -1;
        },
    };

    let statbuf: &mut stat::stat = &mut *statbuf;

    crate::sys::stat::lstat(pathname, statbuf)
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
/// - [`crate::sys::stat::stat`]
///
/// # Safety
///
/// This function has undefined because it dereferences a raw pointer (ie. `statbuf`).
///
#[no_mangle]
pub unsafe extern "C" fn stat(pathname: *const c_char, statbuf: *mut stat::stat) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("stat(): invalid pathname");
            errno = ErrorCode::InvalidArgument.into_errno();
            return -1;
        },
    };

    let statbuf: &mut stat::stat = &mut *statbuf;

    crate::sys::stat::stat(pathname, statbuf)
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn mkdir(_pathname: *const c_char, _mode: u32) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/347
    ::nvx::error!("mkdir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    -1
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn truncate(_path: *const c_char, _length: u64) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/454
    ::nvx::error!("truncate(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    -1
}
