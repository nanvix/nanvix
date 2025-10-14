// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmdir(path: *const c_char) -> c_int {
    ::syslog::trace!("rmdir(): path={path:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/348
    ::syslog::debug!("rmdir(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}
