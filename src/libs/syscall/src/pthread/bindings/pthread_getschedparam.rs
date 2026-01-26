// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sched::sched_param,
    sys_types::pthread_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_getschedparam(
    thread: pthread_t,
    policy: *mut c_int,
    param: *mut sched_param,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/725
    ::syslog::debug!("pthread_getschedparam(): not implemented");
    0
}
