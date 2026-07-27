// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::ErrorCode,
    kcall::{
        arch,
        pm::__kcall_get_thread_data_area,
    },
};
use ::sysalloc::tda::TDA_CANCEL_STATE_OFFSET;
use ::sysapi::{
    ffi::c_int,
    pthread::{
        PTHREAD_CANCEL_DISABLE,
        PTHREAD_CANCEL_ENABLE,
    },
};
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
/// - `oldstate`: Storage location for the old cancellability state. This may be null.
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
/// - If `oldstate` is non-null, it points to a valid `c_int` variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int {
    // Check if `state` is not valid.
    if state != PTHREAD_CANCEL_ENABLE && state != PTHREAD_CANCEL_DISABLE {
        ::syslog::warn!("pthread_setcancelstate(): invalid state (state={state})");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if the calling thread has a valid thread data area.
    match __kcall_get_thread_data_area() {
        Ok(tda_ptr) if !tda_ptr.is_null() => {},
        Ok(_) => {
            ::syslog::warn!("pthread_setcancelstate(): thread data area is not configured");
            return ErrorCode::InvalidSysCall.get();
        },
        Err(error) => {
            ::syslog::warn!("pthread_setcancelstate(): failed to get thread data area ({error:?})");
            return error.code.get();
        },
    }

    // Atomically update the cancellation state of the calling thread.
    let previous_state: c_int =
        unsafe { arch::swap_tda_u32(TDA_CANCEL_STATE_OFFSET as u32, state as u32) as c_int };

    // Store the previous cancellation state if requested.
    if !oldstate.is_null() {
        unsafe {
            *oldstate = previous_state;
        }
    }

    0
}
