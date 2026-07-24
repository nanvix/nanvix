// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    pthread::{
        PTHREAD_PROCESS_PRIVATE,
        PTHREAD_PROCESS_SHARED,
    },
    sys_types::pthread_mutexattr_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the process-sharing attribute in a mutex attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the mutex attributes object.
/// - `pshared`: Process-sharing attribute to set.
///
/// # Returns
///
/// The `pthread_mutexattr_setpshared()` function returns `0` on success. On error, it returns an
/// error number.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if `attr` points to a valid `pthread_mutexattr_t` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_mutexattr_setpshared(
    attr: *mut pthread_mutexattr_t,
    pshared: c_int,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::warn!(
            "pthread_mutexattr_setpshared(): invalid pointer to mutex attributes object \
             (attr={attr:p}, pshared={pshared})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `attr` is misaligned.
    if !(attr as usize).is_multiple_of(::core::mem::align_of::<pthread_mutexattr_t>()) {
        ::syslog::warn!(
            "pthread_mutexattr_setpshared(): misaligned pointer to mutex attributes object \
             (attr={attr:p}, pshared={pshared})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if the mutex attributes object is initialized.
    if !(*attr).is_initialized() {
        ::syslog::warn!("pthread_mutexattr_setpshared(): attr is not initialized");
        return ErrorCode::InvalidArgument.get();
    }

    match pshared {
        PTHREAD_PROCESS_PRIVATE => {
            (*attr).set_pshared(pshared);
            0
        },
        PTHREAD_PROCESS_SHARED => {
            ::syslog::warn!("pthread_mutexattr_setpshared(): process-shared mode is not supported");
            ErrorCode::OperationNotSupported.get()
        },
        _ => {
            ::syslog::warn!(
                "pthread_mutexattr_setpshared(): invalid process-sharing attribute \
                 (attr={attr:p}, pshared={pshared})"
            );
            ErrorCode::InvalidArgument.get()
        },
    }
}
