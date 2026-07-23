// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pthread::syscall;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        c_size_t,
        pthread_attr_t,
    },
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the stack size attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the thread attributes object.
/// - `stacksize`: Stack size to set.
///
/// # Return Value
///
/// On success, this function returns zero. Otherwise, it returns a non-zero error code indicating
/// the reason for the failure.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`] if `attr` is null or misaligned.
/// - [`ErrorCode::InvalidArgument`] if `attr` points to an uninitialized thread attributes object.
/// - [`ErrorCode::InvalidArgument`] if `stacksize` is smaller than the minimum thread stack size.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if `attr` points to a valid [`pthread_attr_t`].
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_setstacksize(
    attr: *mut pthread_attr_t,
    stacksize: c_size_t,
) -> c_int {
    // Check if `attr` points to an invalid address.
    if attr.is_null() {
        ::syslog::warn!(
            "pthread_attr_setstacksize(): invalid pointer to thread attributes object \
             (attr={attr:p}, stacksize={stacksize})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `attr` is misaligned.
    if !(attr as usize).is_multiple_of(core::mem::align_of::<pthread_attr_t>()) {
        ::syslog::warn!(
            "pthread_attr_setstacksize(): misaligned pointer to thread attributes object \
             (attr={attr:p}, stacksize={stacksize})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Attempt to set the stack size attribute and check for errors.
    match syscall::pthread_attr_setstacksize(&mut *attr, stacksize) {
        Ok(()) => {
            ::syslog::trace!("pthread_attr_setstacksize(): success");
            0
        },
        Err(error) => {
            ::syslog::warn!("pthread_attr_setstacksize(): {error:?}");
            error.code.get()
        },
    }
}
