// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use core::ffi;

use nvx::sys::error::ErrorCode;

use crate::{
    ffi::{
        c_char,
        c_int,
    },
    sys::stat,
};

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
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
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
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };

    let statbuf: &mut stat::stat = &mut *statbuf;

    crate::sys::stat::stat(pathname, statbuf)
}
