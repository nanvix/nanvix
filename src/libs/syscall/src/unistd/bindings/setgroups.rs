// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        c_size_t,
        gid_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub extern "C" fn setgroups(size: c_size_t, list: *const gid_t) -> c_int {
    ::syslog::trace!("setgroups(): size={size}, list={list:p}");
    // TODO: https://github.com/nanvix/nanvix/issues/523
    ::syslog::debug!("setgroups(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
