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
pub unsafe extern "C" fn pthread_sigmask(
    how: c_int,
    set: *const c_void,
    oldset: *mut c_void,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/717
    ::syslog::debug!("pthread_sigmask(): not implemented");
    0
}
