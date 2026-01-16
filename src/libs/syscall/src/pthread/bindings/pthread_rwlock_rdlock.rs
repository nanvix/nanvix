// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_rwlock_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Acquires a read lock on a read-write lock.
///
/// # Parameters
///
/// - `rwlock`: Read-write lock object.
///
/// # Returns
///
/// If successful, this function returns zero. Otherwise, it returns a non-zero error code.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `rwlock` points to a valid `pthread_rwlock_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_rwlock_rdlock(rwlock: *mut pthread_rwlock_t) -> c_int {
    // Check if `rwlock` object is invalid.
    if rwlock.is_null() {
        ::syslog::error!("pthread_rwlock_rdlock(): invalid read-write lock (rwlock={rwlock:p})");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `rwlock` is unaligned.
    if !(rwlock as usize).is_multiple_of(align_of::<pthread_rwlock_t>()) {
        ::syslog::error!("pthread_rwlock_rdlock(): unaligned read-write lock (rwlock={rwlock:p})");
        return ErrorCode::InvalidArgument.get();
    }

    // Attempt to acquire a read lock on the read-write lock and check for errors.
    match crate::pthread::pthread_rwlock_rdlock(&mut *rwlock) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("pthread_rwlock_rdlock(): {error:?} (rwlock={rwlock:p})");
            error.code.get()
        },
    }
}
