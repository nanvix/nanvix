// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_attr_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes a thread attributes object with default values.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
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
/// - `attr` points to a valid `pthread_attr_t` structure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_attr_init(attr: *mut pthread_attr_t) -> c_int {
    ::syslog::trace!("pthread_attr_init(): attr={:?}", attr);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_init(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    *attr = pthread_attr_t::default();

    0
}
