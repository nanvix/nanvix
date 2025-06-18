// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syscall::dirent::DirectoryStream;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dirfd(dirp: *mut DirectoryStream) -> c_int {
    ::syslog::trace!("dirfd(): dirp={dirp:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/596
    ::syslog::error!("dirfd(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
