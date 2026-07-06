// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::sem_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Unlocks the unnamed semaphore referred to by `sem`, incrementing its value and waking up a
/// blocked waiter, if any.
///
/// # Parameters
///
/// - `sem`: Semaphore to unlock.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, `-1` is returned and `errno` is set to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `sem` points to a valid `sem_t` structure that was previously initialized by `sem_init()`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn sem_post(sem: *mut sem_t) -> c_int {
    // Check if `sem` is not valid.
    if sem.is_null() {
        ::syslog::warn!("sem_post(): invalid semaphore pointer");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    match crate::pthread::sem_post(&mut *sem) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::warn!("sem_post(): {error:?}");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
