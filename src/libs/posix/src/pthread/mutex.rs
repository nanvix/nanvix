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
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
    time::timespec,
};
use ::syscall::pthread;
use ::syslog::trace_libcall;

//==================================================================================================
// pthread_mutexattr_init()
//==================================================================================================

///
/// # Description
///
/// Initializes a mutex attributes object with default values.
///
/// # Parameters
///
/// - `attr`: Pointer to the mutex attributes object to be initialized.
///
/// # Returns
///
/// The `pthread_mutexattr_init()` function returns `0` on success. On error, it returns an error number.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `attr` points to a valid `pthread_mutexattr_t` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_mutexattr_init(attr: *mut pthread_mutexattr_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/511.
    0
}

//==================================================================================================
// pthread_mutexattr_destroy()
//==================================================================================================

///
/// # Description
///
/// Destroys a mutex attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the mutex attributes object to be destroyed.
///
/// # Returns
///
/// The `pthread_mutexattr_destroy()` function returns `0` on success. On error, it returns an error number.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `attr` points to a valid `pthread_mutexattr_t` object.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_mutexattr_destroy(attr: *mut pthread_mutexattr_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/509
    0
}

//==================================================================================================
// pthread_mutex_timedlock()
//==================================================================================================

///
/// # Description
///
/// Locks a mutex with a timeout.
///
/// # Parameters
///
/// - `mutex`: Mutex object.
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
///
/// - `mutex` points to a valid `pthread_mutex_t` structure.
/// - `abstime` points to a valid `timespec` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_mutex_timedlock(
    mutex: *mut pthread_mutex_t,
    abstime: *const timespec,
) -> c_int {
    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::syslog::error!("pthread_mutex_timedlock(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `abstime` is not valid.
    if abstime.is_null() {
        ::syslog::error!("pthread_mutex_timedlock(): invalid abstime pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Try to convert the `abstime`.
    let timeout: SystemTime =
        match SystemTime::new((*abstime).tv_sec as u64, (*abstime).tv_nsec as u32) {
            Some(timeout) => timeout,
            None => {
                ::syslog::error!(
                    "pthread_mutex_timedlock(): invalid timeout (abstime={:?})",
                    abstime
                );
                return ErrorCode::InvalidArgument.get();
            },
        };

    match pthread::pthread_mutex_timedlock(&mut *mutex, Some(timeout)) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::error!(
                "pthread_mutex_timedlock(): failed to lock mutex (abstime={:?}, error={:?})",
                abstime,
                error
            );
            error.code.get()
        },
    }
}

//==================================================================================================
// pthread_mutex_trylock()
//==================================================================================================

///
/// # Description
///
/// Tries to lock a mutex.
///
/// # Parameters
///
/// - `mutex`: Mutex object.
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
///
/// - `mutex` points to a valid `pthread_mutex_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_mutex_trylock(mutex: *mut pthread_mutex_t) -> c_int {
    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::syslog::error!("pthread_mutex_trylock(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match pthread::pthread_mutex_trylock(&mut *mutex) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("pthread_mutex_trylock(): failed to lock mutex (error={:?})", error);
            error.code.get()
        },
    }
}
