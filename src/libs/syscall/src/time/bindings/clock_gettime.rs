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
/// The `clock_gettime()` function shall return the current value of the specified clock `clock_id`.
///
/// # Parameters
///
/// - `clock_id`: The identifier of the clock to be used.
/// - `tp`: The structure where the time is stored.
///
/// # Returns
///
/// The `clock_gettime()` function shall return 0 upon successful completion. Otherwise, it shall
/// return -1 and set `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may deference raw pointers.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clock_gettime(clock_id: clockid_t, tp: *mut timespec) -> c_int {
    let mut tp: Option<&mut timespec> = if tp.is_null() {
        None
    } else {
        Some(unsafe { &mut *tp })
    };
    match crate::time::clock_gettime(clock_id, &mut tp) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::error!(
                "clock_gettime(): failed (clock_id={:?}, tp={:?}, error={:?})",
                clock_id,
                tp,
                error
            );
            // Set errno.
            *__errno_location() = error.code.get();
            -1
        },
    }
}
