// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        clockid_t,
        pthread_condattr_t,
    },
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the clock attribute in a condition variable attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the condition variable attributes object.
/// - `clock_id`: The clock identifier to set.
///
/// # Returns
///
/// Upon successful completion, returns 0. Otherwise, returns an error number.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if and only if:
/// - `attr` is a valid pointer to a `pthread_condattr_t` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_condattr_setclock(
    attr: *mut pthread_condattr_t,
    clock_id: clockid_t,
) -> c_int {
    // TODO (#500): implement pthread_condattr_setclock
    ::syslog::debug!(
        "pthread_condattr_setclock(): not implemented (attr={attr:?}, clock_id={clock_id:?})"
    );
    ErrorCode::InvalidSysCall.get()
}
