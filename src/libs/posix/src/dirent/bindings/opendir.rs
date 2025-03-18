// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::DirectoryStream,
    errno::errno,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn opendir(_name: *const u8) -> *mut DirectoryStream {
    ::nvx::error!("opendir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    core::ptr::null_mut()
}
