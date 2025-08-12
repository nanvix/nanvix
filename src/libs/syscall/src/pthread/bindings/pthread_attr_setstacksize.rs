// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_attr_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// TODO: add description
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_attr_setstacksize(
    _attr: *mut pthread_attr_t,
    _stacksize: usize,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/488
    ::syslog::warn!("pthread_attr_setstacksize(): not implemented");
    0
}
