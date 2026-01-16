// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_int,
    c_void,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn sem_post(sem: *mut c_void) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/723
    ::syslog::debug!("sem_post(): not implemented");
    0
}
