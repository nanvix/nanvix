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
    ffi::c_int,
};
use ::alloc::boxed::Box;
use nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn closedir(dirp: *mut DirectoryStream) -> c_int {
    // Check if directory stream is invalid.
    if dirp.is_null() {
        ::nvx::error!("closedir(): invalid directory stream");
        errno = ErrorCode::InvalidArgument.into_errno();
        return -1;
    }

    ::nvx::trace!("closedir(): dirp={:?}", dirp);

    let mut dirp: Box<DirectoryStream> = Box::from_raw(dirp);

    // Close directory stream and check for errors.
    match dirent::closedir(&mut dirp) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("closedir(): failed to close directory stream: {:?}", error);
            errno = error.code.into_errno();
            -1
        },
    }
}
