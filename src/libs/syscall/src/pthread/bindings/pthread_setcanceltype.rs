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
use ::sysalloc::tda::TDA_CANCEL_TYPE_OFFSET;
use ::sysapi::{
    ffi::c_int,
    pthread::{
        PTHREAD_CANCEL_ASYNCHRONOUS,
        PTHREAD_CANCEL_DEFERRED,
    },
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the cancellability type of the calling thread.
///
/// # Parameters
///
/// - `type_`: New cancellability type.
/// - `oldtype`: Storage location for the old cancellability type. This may be null.
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
/// - If `oldtype` is non-null, it points to a valid `c_int` variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_setcanceltype(type_: c_int, oldtype: *mut c_int) -> c_int {
    // Check if `type_` is not valid.
    if type_ != PTHREAD_CANCEL_DEFERRED && type_ != PTHREAD_CANCEL_ASYNCHRONOUS {
        ::syslog::warn!("pthread_setcanceltype(): invalid type (type={type_})");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if the calling thread has a valid thread data area.
    match __kcall_get_thread_data_area() {
        Ok(tda_ptr) if !tda_ptr.is_null() => {},
        Ok(_) => {
            ::syslog::warn!("pthread_setcanceltype(): thread data area is not configured");
            return ErrorCode::InvalidSysCall.get();
        },
        Err(error) => {
            ::syslog::warn!("pthread_setcanceltype(): failed to get thread data area ({error:?})");
            return error.code.get();
        },
    }

    // Atomically update the cancellation type of the calling thread. Cancellation requests are not
    // supported, so no request can be pending for the asynchronous type to act upon right away.
    let previous_type: c_int =
        unsafe { arch::swap_tda_u32(TDA_CANCEL_TYPE_OFFSET as u32, type_ as u32) as c_int };

    // Store the previous cancellation type if requested.
    if !oldtype.is_null() {
        unsafe {
            *oldtype = previous_type;
        }
    }

    0
}
