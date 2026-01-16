// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_mutex_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Destroys a mutex.
///
/// # Parameters
///
/// - `mutex`: Mutex object.
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
/// - `mutex` points to a valid `pthread_mutex_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> c_int {
    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::syslog::error!("pthread_mutex_destroy(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    if let Err(error) = crate::pthread::pthread_mutex_destroy(&mut *mutex) {
        return error.code.get();
    }

    0
}
