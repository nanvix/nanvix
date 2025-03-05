// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    sys::{
        types::ssize_t,
        uio::iovec,
    },
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn writev(_fd: i32, _iov: *const iovec, _iovcnt: i32) -> ssize_t {
    // TODO: https://github.com/nanvix/nanvix/issues/288
    ::nvx::error!("writev() not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.into_errno();
    }
    -1
}
