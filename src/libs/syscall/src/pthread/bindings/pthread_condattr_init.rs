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
/// Initializes a condition variable attributes object with default values.
///
/// # Parameters
///
/// - `attr`: Pointer to the condition variable attributes object to initialize.
///
/// # Return Value
///
/// On success, returns 0. Otherwise, returns an error code.
///
/// # Notes
///
/// This is a dummy implementation. It only validates the input pointer and then returns success.
/// A full implementation should set default values and mark the object as initialized.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// It is safe to call this function iff `attr` points to writable memory large enough to hold a
/// `pthread_condattr_t` structure and is properly aligned.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_condattr_init(attr: *mut pthread_condattr_t) -> c_int {
    // Check if pointer to cond attribute object is valid.
    if attr.is_null() {
        ::syslog::error!(
            "pthread_condattr_init(): invalid pointer to cond attribute object (attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if pointer to cond attribute object is properly aligned.
    if !(attr as usize).is_multiple_of(::core::mem::align_of::<pthread_condattr_t>()) {
        ::syslog::error!(
            "pthread_condattr_init(): misaligned pointer to cond attribute object (attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Dummy initialization: write default structure.
    ::core::ptr::write(attr, pthread_condattr_t::default());
    ::syslog::warn!("pthread_condattr_init(): dummy implementation (attr={attr:p})");

    0
}
