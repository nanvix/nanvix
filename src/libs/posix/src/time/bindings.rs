// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports

use crate::{
    errno::errno,
    ffi::c_int,
    sys::types::clockid_t,
    time::timespec,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn clock_getres(_clock_id: clockid_t, _res: *mut timespec) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/274
    ::nvx::error!("clock_getres(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
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
            unsafe { errno = e.code.get() };
            -1
        },
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn nanosleep(_req: *const u8, _rem: *mut u8) -> c_int {
    ::nvx::trace!("nanosleep(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}
