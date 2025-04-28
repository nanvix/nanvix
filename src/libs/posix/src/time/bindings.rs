// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports

use crate::{
    errno::__errno_location,
    ffi::c_int,
    sys::types::clockid_t,
    time::timespec,
};
use ::nvx::sys::error::ErrorCode;

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
/// Upon successful completion, `clock_getres()` returns zero. Otherwise, it returns `-1`` and sets
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
#[no_mangle]
pub unsafe extern "C" fn clock_getres(clock_id: clockid_t, res: *mut timespec) -> c_int {
    ::nvx::trace!("clock_getres(): clock_id={:?}, res={:?}", clock_id, res);

    // Convert `res` pointer to a reference.
    let mut res: Option<&mut timespec> = if res.is_null() { None } else { Some(&mut *res) };

    // Get clock resolution and parse the result.
    match crate::time::clock_getres(clock_id, &mut res) {
        // Systemc all succeeded.
        Ok(()) => 0,
        // System call failed.
        Err(error) => {
            ::nvx::error!(
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
#[no_mangle]
pub unsafe extern "C" fn clock_gettime(clock_id: clockid_t, tp: *mut timespec) -> c_int {
    let tp: Option<&mut timespec> = if tp.is_null() {
        None
    } else {
        Some(unsafe { &mut *tp })
    };
    match crate::time::clock_gettime(clock_id, tp) {
        Ok(_) => 0,
        Err(e) => {
            // Set errno.
            *__errno_location() = e.code.get();
            -1
        },
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn nanosleep(_req: *const u8, _rem: *mut u8) -> c_int {
    ::nvx::trace!("nanosleep(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}
