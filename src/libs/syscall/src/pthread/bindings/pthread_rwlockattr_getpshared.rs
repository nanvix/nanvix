// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_rwlockattr_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the process-sharing attribute from a read-write lock attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the read-write lock attributes object.
/// - `pshared`: Pointer where the process-sharing attribute is stored on success.
///
/// # Returns
///
/// The `pthread_rwlockattr_getpshared()` function returns `0` on success. On error, it returns an
/// error number.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `attr` points to a valid `pthread_rwlockattr_t` object.
/// - `pshared` points to a valid `int` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_rwlockattr_getpshared(
    attr: *const pthread_rwlockattr_t,
    pshared: *mut c_int,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::warn!(
            "pthread_rwlockattr_getpshared(): invalid pointer to read-write lock attributes \
             object (attr={attr:p}, pshared={pshared:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `attr` is misaligned.
    if !(attr as usize).is_multiple_of(::core::mem::align_of::<pthread_rwlockattr_t>()) {
        ::syslog::warn!(
            "pthread_rwlockattr_getpshared(): misaligned pointer to read-write lock attributes \
             object (attr={attr:p}, pshared={pshared:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `pshared` is not valid.
    if pshared.is_null() {
        ::syslog::warn!(
            "pthread_rwlockattr_getpshared(): invalid pointer to process-sharing attribute \
             (attr={attr:p}, pshared={pshared:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `pshared` is misaligned.
    if !(pshared as usize).is_multiple_of(::core::mem::align_of::<c_int>()) {
        ::syslog::warn!(
            "pthread_rwlockattr_getpshared(): misaligned pointer to process-sharing attribute \
             (attr={attr:p}, pshared={pshared:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if the read-write lock attributes object is initialized.
    if !(*attr).is_initialized() {
        ::syslog::warn!(
            "pthread_rwlockattr_getpshared(): attr is not initialized (attr={attr:p}, \
             pshared={pshared:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    *pshared = (*attr).pshared();

    0
}
