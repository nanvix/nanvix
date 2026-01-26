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

// TODO: add description
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn sem_init(sem: *mut c_void, pshared: c_int, value: u32) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/721
    ::syslog::debug!("sem_init(): not implemented");
    0
}
