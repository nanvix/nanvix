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
/// Destroys a read-write lock attributes object. A destroyed attributes object may be initialized
/// again with `pthread_rwlockattr_init()`. Destroying an attributes object does not affect
/// read-write locks that were previously initialized with it.
///
/// # Parameters
///
/// - `attr`: Pointer to the read-write lock attributes object to destroy.
///
/// # Returns
///
/// The `pthread_rwlockattr_destroy()` function returns `0` on success. On error, it returns an
/// error number.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if `attr` points to a valid `pthread_rwlockattr_t` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_rwlockattr_destroy(attr: *mut pthread_rwlockattr_t) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::warn!(
            "pthread_rwlockattr_destroy(): invalid pointer to read-write lock attributes object \
             (attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `attr` is misaligned.
    if !(attr as usize).is_multiple_of(::core::mem::align_of::<pthread_rwlockattr_t>()) {
        ::syslog::warn!(
            "pthread_rwlockattr_destroy(): misaligned pointer to read-write lock attributes \
             object (attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if the read-write lock attributes object is initialized.
    if !(*attr).is_initialized() {
        ::syslog::warn!("pthread_rwlockattr_destroy(): attr is not initialized (attr={attr:p})");
        return ErrorCode::InvalidArgument.get();
    }

    (*attr).uninitialize();

    0
}
