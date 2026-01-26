// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::ErrorCode,
    time::SystemTime,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        pthread_cond_t,
        pthread_mutex_t,
    },
    time::timespec,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Waits on a condition variable with a timeout.
///
/// # Parameters
///
/// - `cond`: Condition variable.
/// - `mutex`: Mutex object.
/// - `abstime`: Absolute time to wait until.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `cond` points to a valid `pthread_cond_t` structure.
/// - `mutex` points to a valid `pthread_mutex_t` structure.
/// - `abstime` points to a valid `timespec` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_cond_timedwait(
    cond: *const pthread_cond_t,
    mutex: *mut pthread_mutex_t,
    abstime: *const timespec,
) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_timedwait(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::syslog::error!("pthread_cond_timedwait(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `abstime` is not valid.
    if abstime.is_null() {
        ::syslog::error!("pthread_cond_timedwait(): invalid absolute time pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Try to convert `abstime`.
    let timeout: SystemTime =
        match SystemTime::new((*abstime).tv_sec as u64, (*abstime).tv_nsec as u32) {
            Some(timeout) => timeout,
            None => {
                ::syslog::error!(
                    "pthread_cond_timedwait(): invalid timeout (cond={:?}, mutex={:?}, \
                     abstime={:?})",
                    cond,
                    mutex,
                    abstime
                );
                return ErrorCode::InvalidArgument.get();
            },
        };

    match crate::pthread::pthread_cond_timedwait(&*cond, &*mutex, Some(timeout)) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "pthread_cond_timedwait(): failed to wait on condition variable (cond={:?}, \
                 mutex={:?}, abstime={:?}, error={:?})",
                cond,
                mutex,
                abstime,
                error
            );
            error.code.get()
        },
    }
}
