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
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    ::nvx::trace!("closedir(): dirp={:?}", dirp);

    let mut dirp: Box<DirectoryStream> = Box::from_raw(dirp);

    // Close directory stream and check for errors.
    match dirent::closedir(&mut dirp) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("closedir(): failed to close directory stream: {:?}", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}
