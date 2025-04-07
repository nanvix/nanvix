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
    limits::NAME_MAX,
};
use ::alloc::boxed::Box;
use ::core::ptr;
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Directory entry returned by `readdir()`.
static mut DIRENT: dirent::dirent = dirent::dirent {
    d_ino: 0,
    d_name: [0; NAME_MAX + 1],
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn readdir(dirp: *mut DirectoryStream) -> *mut dirent::dirent {
    // Check if directory stream is invalid.
    if dirp.is_null() {
        ::nvx::error!("closedir(): invalid directory stream");
        errno = ErrorCode::InvalidArgument.get();
        return ptr::null_mut();
    }

    ::nvx::trace!("readdir(): dirp={:?}", dirp);

    let mut dirp: Box<DirectoryStream> = Box::from_raw(dirp);

    // Read directory entry and check for errors.
    let direntp: *mut dirent::dirent = match dirent::readdir(&mut dirp) {
        // End of directory.
        Ok(None) => ptr::null_mut(),
        // Directory entry read.
        Ok(Some(dirent)) => {
            DIRENT.d_ino = dirent.d_ino;
            DIRENT.d_name = dirent.d_name;
            &raw mut DIRENT
        },
        // Error.
        Err(error) => {
            ::nvx::error!(
                "readdir(): failed to read directory entry (dirp={:?}, error={:?})",
                dirp,
                error
            );
            errno = error.code.get();
            ptr::null_mut()
        },
    };

    // Leak the directory stream to keep it alive for future operations.
    let _dirp: *mut DirectoryStream = Box::into_raw(dirp);

    direntp
}
