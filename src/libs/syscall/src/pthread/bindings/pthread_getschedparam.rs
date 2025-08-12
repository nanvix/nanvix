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

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_getschedparam(
    _thread: pthread_t,
    _policy: *mut c_int,
    _param: *mut sched_param,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/725
    ::syslog::warn!("pthread_getschedparam(): not implemented");
    0
}
