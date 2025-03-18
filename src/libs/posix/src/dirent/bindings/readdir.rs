// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::{
        dirent,
        DirectoryStream,
    },
    errno::errno,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn readdir(_dirp: *mut DirectoryStream) -> *mut dirent {
    // TODO: https://github.com/nanvix/nanvix/issues/522
    ::nvx::error!("readdir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    core::ptr::null_mut()
}
