// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the cancellability state of the calling thread.
///
/// # Parameters
///
/// - `state`: New cancellability state.
/// - `oldstate`: Old cancellability state.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `oldstate` points to a valid `c_int` variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int {
    // Check if `oldstate` is not valid.
    if oldstate.is_null() {
        ::syslog::error!("pthread_setcancelstate(): invalid old state pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::debug!("pthread_setcancelstate(): not supported, ignoring");
    0
}
