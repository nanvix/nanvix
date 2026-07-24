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
/// Gets the clock attribute from a condition variable attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the condition variable attributes object.
/// - `clock_id`: Pointer where the clock attribute is stored on success.
///
/// # Returns
///
/// The `pthread_condattr_getclock()` function returns `0` on success. On error, it returns an error
/// number.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `attr` points to a valid `pthread_condattr_t` object.
/// - `clock_id` points to a valid `clockid_t` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_condattr_getclock(
    attr: *const pthread_condattr_t,
    clock_id: *mut clockid_t,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::warn!(
            "pthread_condattr_getclock(): invalid pointer to condition variable attributes object \
             (attr={attr:p}, clock_id={clock_id:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `attr` is misaligned.
    if !(attr as usize).is_multiple_of(::core::mem::align_of::<pthread_condattr_t>()) {
        ::syslog::warn!(
            "pthread_condattr_getclock(): misaligned pointer to condition variable attributes \
             object (attr={attr:p}, clock_id={clock_id:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `clock_id` is not valid.
    if clock_id.is_null() {
        ::syslog::warn!(
            "pthread_condattr_getclock(): invalid pointer to clock attribute (attr={attr:p}, \
             clock_id={clock_id:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `clock_id` is misaligned.
    if !(clock_id as usize).is_multiple_of(::core::mem::align_of::<clockid_t>()) {
        ::syslog::warn!(
            "pthread_condattr_getclock(): misaligned pointer to clock attribute (attr={attr:p}, \
             clock_id={clock_id:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if the condition variable attributes object is initialized.
    if !(*attr).is_initialized() {
        ::syslog::warn!("pthread_condattr_getclock(): attr is not initialized");
        return ErrorCode::InvalidArgument.get();
    }

    *clock_id = (*attr).clock();
    0
}
