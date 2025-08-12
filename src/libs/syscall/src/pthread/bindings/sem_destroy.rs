// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_int,
    c_void,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// TODO: add description
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sem_destroy(_sem: *mut c_void) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/722
    ::syslog::warn!("sem_destroy(): not implemented");
    0
}
