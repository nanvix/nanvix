// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// TODO: add description
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_kill(_thread: pthread_t, _sig: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/716
    ::syslog::debug!("pthread_kill(): not implemented");
    0
}
