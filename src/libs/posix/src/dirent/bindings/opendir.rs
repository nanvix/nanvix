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
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendir(dirname: *const i8) -> *mut DirectoryStream {
    // Convert C string to Rust string.
    let dirname: &str = match ffi::CStr::from_ptr(dirname).to_str() {
        Ok(dirname) => dirname,
        Err(_) => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return ptr::null_mut();
        },
    };

    ::syslog::trace!("opendir(): dirname={:?}", dirname);

    // Open directory stream and check for errors.
    let dirp: Box<DirectoryStream> = match dirent::opendir(dirname) {
        Ok(dirp) => dirp,
        Err(error) => {
            ::syslog::error!("opendir(): failed to open directory stream: {:?}", error);
            *__errno_location() = error.code.get();
            return ptr::null_mut();
        },
    };

    Box::into_raw(dirp)
}
