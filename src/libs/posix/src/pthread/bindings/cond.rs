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
use ::nvx::sys::error::ErrorCode;

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
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_broadcast(cond: *const pthread_cond_t) -> c_int {
    ::nvx::trace!("pthread_cond_broadcast(): cond={:?}", cond);

    // Check if `cond` is not valid.
    if cond.is_null() {
        ::nvx::error!("pthread_cond_broadcast(): invalid condition variable pointer");
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
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_init(
    cond: *mut pthread_cond_t,
    attr: *const pthread_condattr_t,
) -> c_int {
    ::nvx::trace!("pthread_cond_init(): cond={:?}, attr={:?}", cond, attr);

    // Check if `cond` is not valid.
    if cond.is_null() {
        ::nvx::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if we should use custom attributes.
    if !attr.is_null() {
        ::nvx::error!(
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
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_destroy(cond: *mut pthread_cond_t) -> c_int {
    ::nvx::trace!("pthread_cond_destroy(): cond={:?}", cond);

    // Check if `cond` is not valid.
    if cond.is_null() {
        ::nvx::error!("pthread_cond_broadcast(): invalid condition variable pointer");
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
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_signal(cond: *const pthread_cond_t) -> c_int {
    ::nvx::trace!("pthread_cond_signal(): cond={:?}", cond);

    // Check if `cond` is not valid.
    if cond.is_null() {
        ::nvx::error!("pthread_cond_broadcast(): invalid condition variable pointer");
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

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_timedwait(
    _cond: *const pthread_cond_t,
    _mutex: *mut pthread_mutex_t,
    _abstime: *const timespec,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/494
    ::nvx::error!("pthread_cond_timedwait(): not implemented");
    ErrorCode::InvalidSysCall.get()
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
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_wait(
    cond: *const pthread_cond_t,
    mutex: *mut pthread_mutex_t,
) -> c_int {
    ::nvx::trace!("pthread_cond_wait(): cond={:?}, mutex={:?}", cond, mutex);

    // Check if `cond` is not valid.
    if cond.is_null() {
        ::nvx::error!("pthread_cond_broadcast(): invalid condition variable pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::nvx::error!("pthread_cond_broadcast(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match syscall::pthread_cond_wait(&*cond, &*mutex) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}
