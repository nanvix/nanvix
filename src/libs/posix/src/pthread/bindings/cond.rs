// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    pthread::{
        pthread_mutex_t,
        syscall,
    },
    sys::types::{
        pthread_cond_t,
        pthread_condattr_t,
    },
    time::timespec,
};
use ::nvx::sys::{
    error::ErrorCode,
    time::SystemTime,
};

//==================================================================================================
// pthread_cond_broadcast()
//==================================================================================================

///
/// # Description
///
/// Broadcasts a condition variable, waking up all threads waiting on it.
///
/// # Parameters
///
/// - `cond`: Condition variable.
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
/// - `cond` points to a valid `pthread_cond_t` structure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_cond_broadcast(cond: *const pthread_cond_t) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match syscall::pthread_cond_broadcast(&*cond) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}

//==================================================================================================
// pthread_cond_init()
//==================================================================================================

///
/// # Description
///
/// Initializes a condition variable.
///
/// # Parameters
///
/// - `cond`: Condition variable.
/// - `attr`: Condition variable attributes.
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
/// - `cond` points to a valid `pthread_cond_t` structure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_cond_init(
    cond: *mut pthread_cond_t,
    attr: *const pthread_condattr_t,
) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if we should use custom attributes.
    if !attr.is_null() {
        ::syslog::error!(
            "pthread_cond_broadcast(): condition variable attributes are not supported, ignoring"
        );
    }

    // TODO: once we support custom attributes, dereference that pointer.
    let attr: pthread_condattr_t = pthread_condattr_t::default();

    match syscall::pthread_cond_init(&mut *cond, &attr) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}

//==================================================================================================
// pthread_cond_destroy()
//==================================================================================================

///
/// # Description
///
/// Destroys a condition variable.
///
/// # Parameters
///
/// - `cond`: Condition variable.
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
/// - `cond` points to a valid `pthread_cond_t` structure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_cond_destroy(cond: *mut pthread_cond_t) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match syscall::pthread_cond_destroy(&mut *cond) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}

//==================================================================================================
// pthread_cond_signal()
//==================================================================================================

///
/// # Description
///
/// Signals a condition variable, waking up one thread waiting on it.
///
/// # Parameters
///
/// - `cond`: Condition variable.
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
/// - `cond` points to a valid `pthread_cond_t` structure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_cond_signal(cond: *const pthread_cond_t) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match syscall::pthread_cond_signal(&*cond) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}

//==================================================================================================
// pthread_cond_timedwait()
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
pub unsafe extern "C" fn pthread_cond_timedwait(
    cond: *const pthread_cond_t,
    mutex: *mut pthread_mutex_t,
    abstime: *const timespec,
) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `abstime` is not valid.
    if abstime.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid absolute time pointer");
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

    match syscall::pthread_cond_timedwait(&*cond, &*mutex, Some(timeout)) {
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

//==================================================================================================
// pthread_cond_wait()
//==================================================================================================

///
/// # Description
///
/// Waits on a condition variable.
///
/// # Parameters
///
/// - `cond`: Condition variable.
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
/// - `cond` points to a valid `pthread_cond_t` structure.
/// - `mutex` points to a valid `pthread_mutex_t` structure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_cond_wait(
    cond: *const pthread_cond_t,
    mutex: *mut pthread_mutex_t,
) -> c_int {
    // Check if `cond` is not valid.
    if cond.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::syslog::error!("pthread_cond_broadcast(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match syscall::pthread_cond_wait(&*cond, &*mutex) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}
