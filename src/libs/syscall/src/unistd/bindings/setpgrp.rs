// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the process group. Nanvix does not implement process groups, so this call has no
/// observable side effects and always reports success. It exists so that portable software that
/// detaches from a controlling terminal compiles, links, and runs.
///
/// # Returns
///
/// `0`.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn setpgrp() -> c_int {
    0
}
