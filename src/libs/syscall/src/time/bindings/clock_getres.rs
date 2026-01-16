// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    time::timespec,
};
use ::syslog::trace_syscall;
use sysapi::{
    errno::__errno_location,
    sys_types::clockid_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the resolution of the specified clock.
///
/// # Parameters
///
/// - `clock_id`: The clock ID.
/// - `res`: The structure where the resolution is stored.
///
/// # Returns
///
/// Upon successful completion, `clock_getres()` returns zero. Otherwise, it returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may access global variables.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `res` points to a valid `timespec` structure.
/// - This function is not called by multiple threads at the same time.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clock_getres(clock_id: clockid_t, res: *mut timespec) -> c_int {
    // Convert `res` pointer to a reference.
    let mut res: Option<&mut timespec> = if res.is_null() { None } else { Some(&mut *res) };

    // Get clock resolution and parse the result.
    match crate::time::clock_getres(clock_id, &mut res) {
        // System call succeeded.
        Ok(()) => 0,
        // System call failed.
        Err(error) => {
            ::syslog::error!(
                "clock_getres(): failed (clock_id={:?}, res={:?}, error={:?})",
                clock_id,
                res,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
