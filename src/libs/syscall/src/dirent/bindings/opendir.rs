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
use ::core::{
    ffi,
    ptr,
};
use ::sys::error::ErrorCode;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `opendir()` system call opens a directory stream corresponding to the directory named by
/// `dirname`. The directory stream is positioned at the first entry.
///
/// # Parameters
///
/// - `dirname`: Pointer to a null-terminated string containing the pathname of the directory to open.
///
/// # Returns
///
/// Upon successful completion, a pointer to a `DirectoryStream` object is returned. Otherwise, a
/// null pointer is returned and `errno` is set to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer (`dirname`) without guaranteeing
/// its validity. The caller must ensure that:
/// - `dirname` points to a valid, null-terminated C string
/// - The memory pointed to by `dirname` remains valid for the duration of the function call
///
/// The returned pointer is owned by the caller and must be deallocated using the appropriate
/// function (such as `closedir`). Failing to do so will result in a memory leak.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendir(dirname: *const i8) -> *mut DirectoryStream {
    // Check if `dirname` is null.
    if dirname.is_null() {
        ::syslog::error!("opendir(): null dirname (dirname={dirname:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return ptr::null_mut();
    }

    // Convert C string to Rust string.
    let dirname: &str = match ffi::CStr::from_ptr(dirname).to_str() {
        Ok(dirname) => dirname,
        Err(_) => {
            ::syslog::error!("opendir(): invalid dirname (dirname={dirname:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return ptr::null_mut();
        },
    };

    // Open directory stream and check for errors.
    match dirent::opendir(dirname) {
        Ok(dirp) => Box::into_raw(dirp),
        Err(error) => {
            ::syslog::error!("opendir(): {error:?} (dirname={dirname:?})");
            *__errno_location() = error.code.get();
            ptr::null_mut()
        },
    }
}
