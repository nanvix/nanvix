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
    sys_types::pthread_rwlockattr_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the process-sharing attribute in a read-write lock attributes object. Read-write locks are
/// private to the process that creates them, so only `PTHREAD_PROCESS_PRIVATE` is supported.
///
/// # Parameters
///
/// - `attr`: Pointer to the read-write lock attributes object.
/// - `pshared`: Process-sharing attribute to set.
///
/// # Returns
///
/// The `pthread_rwlockattr_setpshared()` function returns `0` on success. On error, it returns an
/// error number and leaves the attributes object unchanged.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if `attr` points to a valid `pthread_rwlockattr_t` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_rwlockattr_setpshared(
    attr: *mut pthread_rwlockattr_t,
    pshared: c_int,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::warn!(
            "pthread_rwlockattr_setpshared(): invalid pointer to read-write lock attributes \
             object (attr={attr:p}, pshared={pshared})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `attr` is misaligned.
    if !(attr as usize).is_multiple_of(::core::mem::align_of::<pthread_rwlockattr_t>()) {
        ::syslog::warn!(
            "pthread_rwlockattr_setpshared(): misaligned pointer to read-write lock attributes \
             object (attr={attr:p}, pshared={pshared})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if the read-write lock attributes object is initialized.
    if !(*attr).is_initialized() {
        ::syslog::warn!(
            "pthread_rwlockattr_setpshared(): attr is not initialized (attr={attr:p}, \
             pshared={pshared})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    match pshared {
        PTHREAD_PROCESS_PRIVATE => {
            (*attr).set_pshared(pshared);
            0
        },
        PTHREAD_PROCESS_SHARED => {
            ::syslog::warn!(
                "pthread_rwlockattr_setpshared(): process-shared mode is not supported \
                 (attr={attr:p}, pshared={pshared})"
            );
            ErrorCode::OperationNotSupported.get()
        },
        _ => {
            ::syslog::warn!(
                "pthread_rwlockattr_setpshared(): invalid process-sharing attribute \
                 (attr={attr:p}, pshared={pshared})"
            );
            ErrorCode::InvalidArgument.get()
        },
    }
}
