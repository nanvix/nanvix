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
pub unsafe extern "C" fn pthread_sigmask(
    _how: c_int,
    _set: *const c_void,
    _oldset: *mut c_void,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/717
    ::syslog::debug!("pthread_sigmask(): not implemented");
    0
}
