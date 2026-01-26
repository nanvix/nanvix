// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_cond_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Broadcasts a condition variable, waking up all threads waiting on it.
///
/// # Parameters
///
/// - `cond`: Condition variable.
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
/// - `cond` points to a valid `pthread_cond_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_cond_broadcast(cond: *const pthread_cond_t) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match crate::pthread::pthread_cond_broadcast(&*cond) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}
