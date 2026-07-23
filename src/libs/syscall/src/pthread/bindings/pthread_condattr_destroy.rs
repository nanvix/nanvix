// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_condattr_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Destroys a condition variable attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the condition variable attributes object to destroy.
///
/// # Return Value
///
/// On success, returns 0. Otherwise, returns an error code.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// It is safe to call this function iff `attr` points to writable memory large enough to hold a
/// `pthread_condattr_t` structure and is properly aligned.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_condattr_destroy(attr: *mut pthread_condattr_t) -> c_int {
    // Check if pointer to cond attribute object is valid.
    if attr.is_null() {
        ::syslog::warn!(
            "pthread_condattr_destroy(): invalid pointer to cond attribute object (attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if pointer to cond attribute object is properly aligned.
    if !(attr as usize).is_multiple_of(::core::mem::align_of::<pthread_condattr_t>()) {
        ::syslog::warn!(
            "pthread_condattr_destroy(): misaligned pointer to cond attribute object \
             (attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if the condition variable attributes object is initialized.
    if !(*attr).is_initialized() {
        ::syslog::warn!("pthread_condattr_destroy(): attr is not initialized");
        return ErrorCode::InvalidArgument.get();
    }

    (*attr).uninitialize();
    0
}
