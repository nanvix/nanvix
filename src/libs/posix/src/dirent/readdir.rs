// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::alloc::boxed::Box;
use ::core::ptr;
use ::sys::error::ErrorCode;
use ::sysapi::{
    dirent::dirent,
    limits::NAME_MAX,
};
use ::syscall::dirent::DirectoryStream;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Directory entry returned by `readdir()`.
static mut DIRENT: dirent = dirent {
    d_ino: 0,
    d_name: [0; NAME_MAX + 1],
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readdir(dirp: *mut DirectoryStream) -> *mut dirent {
    // Check if directory stream is invalid.
    if dirp.is_null() {
        ::syslog::error!("closedir(): invalid directory stream");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return ptr::null_mut();
    }

    ::syslog::trace!("readdir(): dirp={:?}", dirp);

    let mut dirp: Box<DirectoryStream> = Box::from_raw(dirp);

    // Read directory entry and check for errors.
    let direntp: *mut dirent = match syscall::dirent::readdir(&mut dirp) {
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
            ::syslog::error!(
                "readdir(): failed to read directory entry (dirp={:?}, error={:?})",
                dirp,
                error
            );
            *__errno_location() = error.code.get();
            ptr::null_mut()
        },
    };

    // Leak the directory stream to keep it alive for future operations.
    let _dirp: *mut DirectoryStream = Box::into_raw(dirp);

    direntp
}
