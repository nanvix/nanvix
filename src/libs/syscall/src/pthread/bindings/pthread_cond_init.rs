// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        pthread_cond_t,
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
/// Initializes a condition variable.
///
/// # Parameters
///
/// - `cond`: Condition variable.
/// - `attr`: Condition variable attributes.
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
pub unsafe extern "C" fn pthread_cond_init(
    cond: *mut pthread_cond_t,
    attr: *const pthread_condattr_t,
) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_init(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if we should use custom attributes.
    if !attr.is_null() {
        ::syslog::warn!(
            "pthread_cond_init(): condition variable attributes are not supported, ignoring"
        );
    }

    // TODO: once we support custom attributes, dereference that pointer.
    let attr: pthread_condattr_t = pthread_condattr_t::default();

    match crate::pthread::pthread_cond_init(&mut *cond, &attr) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}
