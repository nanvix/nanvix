// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pthread::syscall;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
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
/// Gets the stack address and size attributes in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to thread attributes object.
/// - `stackaddr`: Storage location for the stack address.
/// - `stacksize`: Storage location for the stack size.
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
/// - [`ErrorCode::InvalidArgument`] if `attr` points to a misaligned address
/// - [`ErrorCode::InvalidArgument`] if `stackaddr` points to an invalid address.
/// - [`ErrorCode::InvalidArgument`] if `stackaddr` points to a misaligned address.
/// - [`ErrorCode::InvalidArgument`] if `stacksize` points to an invalid address
/// - [`ErrorCode::InvalidArgument`] if `stacksize` points to a misaligned address.
/// - [`ErrorCode::InvalidArgument`] if `attr` points to a thread attribute object that was not
///   initialized.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
/// - `stackaddr` points to a valid `*mut c_void`.
/// - `stacksize` points to a valid `c_size_t`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_getstack(
    attr: *const pthread_attr_t,
    stackaddr: *mut *mut c_void,
    stacksize: *mut c_size_t,
) -> c_int {
    // Check if `attr` points to an invalid address.
    if attr.is_null() {
        ::syslog::error!(
            "pthread_attr_getstack(): invalid pointer to thread attributes object (attr={attr:p}, \
             stackaddr={stackaddr:p}, stacksize={stacksize:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }
    // Check if `attr` is misaligned.
    if !(attr as usize).is_multiple_of(core::mem::align_of::<pthread_attr_t>()) {
        ::syslog::error!(
            "pthread_attr_getstack(): misaligned pointer to thread attributes object \
             (attr={attr:p}, stackaddr={stackaddr:p}, stacksize={stacksize:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stackaddr` points to an invalid address.
    if stackaddr.is_null() {
        ::syslog::error!(
            "pthread_attr_getstack(): invalid pointer to stack address (attr={attr:p}, \
             stackaddr={stackaddr:p}, stacksize={stacksize:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stackaddr` is misaligned.
    if !(stackaddr as usize).is_multiple_of(core::mem::align_of::<*mut c_void>()) {
        ::syslog::error!(
            "pthread_attr_getstack(): misaligned pointer to stack address (attr={attr:p}, \
             stackaddr={stackaddr:p}, stacksize={stacksize:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stacksize` points to an invalid address.
    if stacksize.is_null() {
        ::syslog::error!(
            "pthread_attr_getstack(): invalid pointer to stack size (attr={attr:p}, \
             stackaddr={stackaddr:p}, stacksize={stacksize:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stacksize` is misaligned.
    if !(stacksize as usize).is_multiple_of(core::mem::align_of::<c_size_t>()) {
        ::syslog::error!(
            "pthread_attr_getstack(): misaligned pointer to stack size (attr={attr:p}, \
             stackaddr={stackaddr:p}, stacksize={stacksize:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Attempt to get stack attributes and check for errors.
    match syscall::pthread_attr_getstack(&*attr, &mut *stackaddr, &mut *stacksize) {
        Ok(()) => {
            ::syslog::trace!("pthread_attr_getstack(): success");
            0
        },
        Err(error) => {
            ::syslog::warn!("pthread_attr_getstack(): {error:?}");
            error.code.get()
        },
    }
}
