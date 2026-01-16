// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn usleep(_usec: u32) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/476
    ::syslog::debug!("usleep(): not implemented");
    0
}
