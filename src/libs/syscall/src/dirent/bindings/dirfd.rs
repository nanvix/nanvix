// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::DirectoryStream,
    errno::__errno_location,
};
use ::alloc::boxed::Box;
use ::core::mem::ManuallyDrop;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `dirfd()` system call extracts the file descriptor used by the directory stream `dirp`.
/// This file descriptor can be used with other functions that operate on file descriptors, such as
/// `fstat()` or `fchdir()`. The file descriptor remains valid as long as the directory stream is
/// open and will be automatically closed when `closedir()` is called on the directory stream.
///
/// # Parameters
///
/// - `dirp`: Pointer to a `DirectoryStream` object that was returned by a previous call to `opendir()`.
///
/// # Returns
///
/// Upon successful completion, `dirfd()` returns the file descriptor associated with the
/// directory stream. If an error occurs, -1 is returned and `errno` is set to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer (`dirp`). It is safe to call
/// this function if and only if all the following conditions are met:
/// - `dirp` points to a valid `DirectoryStream` object that was previously returned by `opendir()`
/// - `dirp` has not been closed by a previous call to `closedir()`
/// - `dirp` remains valid for the duration of the function call
///
/// The returned file descriptor should not be closed directly by the caller. It will be
/// automatically closed when `closedir()` is called on the associated directory stream. Closing
/// the file descriptor directly may lead to undefined behavior when using other directory stream
/// functions.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dirfd(dirp: *mut DirectoryStream) -> c_int {
    // Check if directory stream is invalid.
    if dirp.is_null() {
        ::syslog::error!("dirfd(): invalid directory stream");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    let dirp: ManuallyDrop<Box<DirectoryStream>> = ManuallyDrop::new(Box::from_raw(dirp));

    dirp.fd()
}
