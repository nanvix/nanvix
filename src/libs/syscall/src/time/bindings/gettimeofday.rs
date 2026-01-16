// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_select::timeval,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gettimeofday(_tp: *mut timeval, _tz: *mut c_void) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/317
    ::syslog::debug!("gettimeofday(): not implemented");
    0
}
