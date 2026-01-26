// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        pthread_rwlock_t,
        pthread_rwlockattr_t,
    },
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes a read-write lock.
///
/// # Parameters
///
/// - `rwlock`: Read-write lock object.
/// - `attr`: Read-write lock attributes (currently ignored if non-null).
///
/// # Return Value
///
/// If successful, this function returns zero. Otherwise, it returns a non-zero error code.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `rwlock` points to a valid `pthread_rwlock_t` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_rwlock_init(
    rwlock: *mut pthread_rwlock_t,
    attr: *const pthread_rwlockattr_t,
) -> c_int {
    // Check if `rwlock` is invalid.
    if rwlock.is_null() {
        ::syslog::error!(
            "pthread_rwlock_init(): invalid read-write lock (rwlock={rwlock:p}, attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `rwlock` is unaligned.
    if !(rwlock as usize).is_multiple_of(align_of::<pthread_rwlock_t>()) {
        ::syslog::error!(
            "pthread_rwlock_init(): unaligned read-write lock (rwlock={rwlock:p}, attr={attr:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if custom attributes were passed in.
    if !attr.is_null() {
        // Check if `attr` is unaligned.
        if !(attr as usize).is_multiple_of(align_of::<pthread_rwlockattr_t>()) {
            ::syslog::error!(
                "pthread_rwlock_init(): unaligned read-write lock attribute (rwlock={rwlock:p}, \
                 attr={attr:p})"
            );
            return ErrorCode::InvalidArgument.get();
        }

        // TODO (#978): support custom attributes.
        ::syslog::warn!("pthread_rwlock_init(): custom attributes not supported, ignoring");
    }

    let attr: pthread_rwlockattr_t = pthread_rwlockattr_t::default();

    // Attempt to initialize read-write lock and check for errors.
    if let Err(error) = crate::pthread::pthread_rwlock_init(&mut *rwlock, &attr) {
        ::syslog::error!("pthread_rwlock_init(): {error:?} (rwlock={rwlock:p}, attr={attr:?})");
        return error.code.get();
    }

    0
}
