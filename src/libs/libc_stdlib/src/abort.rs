// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
};

//==================================================================================================
// External Declarations
//==================================================================================================

extern "C" {
    fn _exit(status: c_int) -> !;
    fn getpid() -> pid_t;
    fn kill(pid: pid_t, sig: c_int) -> c_int;
}

//==================================================================================================
// Constants
//==================================================================================================

/// Signal number for SIGABRT.
const SIGABRT: c_int = 6;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Causes abnormal process termination by raising `SIGABRT`.
///
/// # Safety
///
/// This function is unsafe because it terminates the process.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/abort.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn abort() -> ! {
    let _ = kill(getpid(), SIGABRT);
    _exit(134)
}
