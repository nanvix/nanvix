// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::alloc::boxed::Box;
use ::nvx::sys::error::ErrorCode;
use ::syscall::{
    dirent::{
        self,
        DirectoryStream,
    },
    ffi::c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn closedir(dirp: *mut DirectoryStream) -> c_int {
    // Check if directory stream is invalid.
    if dirp.is_null() {
        ::syslog::error!("closedir(): invalid directory stream");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    ::syslog::trace!("closedir(): dirp={:?}", dirp);

    let mut dirp: Box<DirectoryStream> = Box::from_raw(dirp);

    // Close directory stream and check for errors.
    match dirent::closedir(&mut dirp) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("closedir(): failed to close directory stream: {:?}", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}
