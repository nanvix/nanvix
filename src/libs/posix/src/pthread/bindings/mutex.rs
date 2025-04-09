// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    pthread::syscall,
    sys::types::{
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// pthread_mutex_destroy()
//==================================================================================================

///
/// # Description
///
/// Destroys a mutex.
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
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> c_int {
    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::nvx::error!("pthread_mutex_destroy(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    if let Err(error) = syscall::pthread_mutex_destroy(&mut *mutex) {
        return error.code.get();
    }

    0
}

//==================================================================================================
// pthread_mutex_init()
//==================================================================================================

///
/// # Description
///
/// Initializes a mutex.
///
/// # Parameters
///
/// - `mutex`: Mutex object.
/// - `attr`: Mutex attributes.
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
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_init(
    mutex: *mut pthread_mutex_t,
    attr: *const pthread_mutexattr_t,
) -> c_int {
    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::nvx::error!("pthread_mutex_init(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if we should use custom attributes.
    if !attr.is_null() {
        ::nvx::warn!("pthread_mutex_init(): custom attributes not supported, ignoring");
    }

    // TODO: once we support custom attributes, dereference that pointer.
    let attr: pthread_mutexattr_t = pthread_mutexattr_t::default();

    if let Err(error) = syscall::pthread_mutex_init(&mut *mutex, &attr) {
        return error.code.get();
    }

    0
}

//==================================================================================================
// pthread_mutex_lock()
//==================================================================================================

///
/// # Description
///
/// Locks a mutex.
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
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_lock(mutex: *mut pthread_mutex_t) -> c_int {
    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::nvx::error!("pthread_mutex_lock(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match syscall::pthread_mutex_lock(&mut *mutex) {
        Ok(_) => 0,
        Err(error) => error.code.get(),
    }
}

//==================================================================================================
// pthread_mutex_trylock()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_trylock(_mutex: *mut pthread_mutex_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/508
    pthread_mutex_lock(_mutex)
}

//==================================================================================================
// pthread_mutex_unlock()
//==================================================================================================

///
/// # Description
///
/// Unlocks a mutex.
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
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_unlock(mutex: *mut pthread_mutex_t) -> c_int {
    // Check if `mutex` is not valid.
    if mutex.is_null() {
        ::nvx::error!("pthread_mutex_unlock(): invalid mutex pointer");
        return ErrorCode::InvalidArgument.get();
    }

    match syscall::pthread_mutex_unlock(&mut *mutex) {
        Ok(_) => 0,
        Err(error) => error.code.get(),
    }
}
