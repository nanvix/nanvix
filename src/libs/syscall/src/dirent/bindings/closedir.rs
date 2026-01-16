// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::{
        self,
        DirectoryStream,
    },
    errno::__errno_location,
};
use ::alloc::boxed::Box;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `closedir()` system call closes the directory stream associated with `dirp`. Upon return,
/// the value of `dirp` may no longer point to an accessible object of the type `DirectoryStream`.
/// If a file descriptor is used to implement type `DirectoryStream`, that file descriptor will be
/// closed.
///
/// # Parameters
///
/// - `dirp`: Pointer to a `DirectoryStream` object that was returned by a previous call to `opendir()`.
///
/// # Returns
///
/// Upon successful completion, `closedir()` returns 0. Otherwise, -1 is returned and `errno` is
/// set to indicate the error. Upon return, `dirp` is no longer valid, regardless of whether the
/// function succeeds or fails.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer (`dirp`).  It is safe to call
/// this function if and only if all the following conditions are met:
/// - `dirp` points to a valid `DirectoryStream` object that was previously returned by `opendir()`
/// - `dirp` has not been previously closed or freed
///
/// Ownership of the pointer is always transferred to this function, regardless of whether it
/// returns success or failure. The pointer must not be reused after this call, as the
/// `DirectoryStream` object will be deallocated and any subsequent use of the pointer results
/// in undefined behavior.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn closedir(dirp: *mut DirectoryStream) -> c_int {
    // Check if directory stream is invalid.
    if dirp.is_null() {
        ::syslog::error!("closedir(): invalid directory stream (dirp={dirp:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    let mut dirp: Box<DirectoryStream> = Box::from_raw(dirp);

    // Close directory stream and check for errors.
    match dirent::closedir(&mut dirp) {
        Ok(()) => 0,
        Err(error) => {
            // We failed, but ownership of `dirp` is now dropped and memory will be freed.
            ::syslog::error!("closedir(): {error:?} (dirp={dirp:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
