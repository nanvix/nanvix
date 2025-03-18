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
    errno::errno,
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
#[no_mangle]
pub unsafe extern "C" fn opendir(dirname: *const i8) -> *mut DirectoryStream {
    // Convert C string to Rust string.
    let dirname: &str = match ffi::CStr::from_ptr(dirname).to_str() {
        Ok(dirname) => dirname,
        Err(_) => {
            errno = ErrorCode::InvalidArgument.into_errno();
            return ptr::null_mut();
        },
    };

    ::nvx::trace!("opendir(): dirname={:?}", dirname);

    // Open directory stream and check for errors.
    let dirp: Box<DirectoryStream> = match dirent::opendir(dirname) {
        Ok(dirp) => dirp,
        Err(error) => {
            ::nvx::error!("opendir(): failed to open directory stream: {:?}", error);
            errno = error.code.into_errno();
            return ptr::null_mut();
        },
    };

    Box::into_raw(dirp)
}
