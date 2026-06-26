// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
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

///
/// # Description
///
/// Sets the system-wide clock. Setting the time of day requires privileges that Nanvix does not
/// grant to user processes, so this always fails with `EPERM`.
///
/// # Parameters
///
/// - `tv`: The structure holding the time the clock should be set to.
/// - `tz`: The (obsolete) timezone argument, ignored.
///
/// # Returns
///
/// Always returns `-1` with `errno` set to `EPERM`.
///
/// # Safety
///
/// This function is unsafe because it accesses the global `errno` location.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn settimeofday(_tv: *const timeval, _tz: *const c_void) -> c_int {
    // Adjusting the system clock is a privileged operation that Nanvix does not expose.
    *__errno_location() = ErrorCode::OperationNotPermitted.get();
    -1
}
