// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pthread::syscall;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_attr_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Destroys a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the thread attributes object to be destroyed.
///
/// # Return Value
///
/// On success, this function returns zero. Otherwise, it returns an non-zero error code indicating
/// the reason for the failure.
///
/// # Errors
///
/// The following errors can be returned by this function:
///
/// - [`ErrorCode::InvalidArgument`] if `attr` points to an invalid address.
/// - [`ErrorCode::InvalidArgument`] if `attr` points to a misaligned address.
/// - [`ErrorCode::InvalidArgument`] if `attr` references a thread attribute object that was not
///   initialized.
///
/// # Notes
///
/// - Destroying a thread attributes object has no effect on threads that were created using that
///   object.
///
/// - This function always succeed, but portable applications should nevertheless handle a possible
///   error return.
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
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_destroy(attr: *mut pthread_attr_t) -> c_int {
    // Check if `attr` is points to an invalid address.
    if attr.is_null() {
        ::syslog::error!(
            "pthread_attr_destroy(): invalid pointer to thread attributes object (attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `attr` points to a misaligned address.
    if !(attr as usize).is_multiple_of(core::mem::align_of::<pthread_attr_t>()) {
        ::syslog::error!(
            "pthread_attr_destroy(): misaligned pointer to thread attributes object \
             (attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Attempt to destroy thread attributes object and check for errors.
    match syscall::pthread_attr_destroy(&mut *attr) {
        Ok(()) => {
            ::syslog::trace!("pthread_attr_destroy(): success (attr={attr:p})");
            0
        },
        Err(error) => {
            ::syslog::warn!("pthread_attr_destroy(): {error:?} (attr={attr:p})");
            error.code.get()
        },
    }
}
