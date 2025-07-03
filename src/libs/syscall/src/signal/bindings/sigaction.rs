// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_int,
    signal::sigaction,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sigaction(
    signum: c_int,
    act: *const sigaction,
    oldact: *mut sigaction,
) -> c_int {
    ::syslog::trace!("sigaction(): signum={signum:?}, act={act:?}, oldact={oldact:?}");

    ::syslog::error!("sigaction(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}
