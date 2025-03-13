// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::{
        c_int,
        c_void,
    },
    pthread::syscall,
    sys::types::pthread_key_t,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
//pthread_key_create()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_key_create(
    key_ptr: *mut pthread_key_t,
    destructor: Option<extern "C" fn(*mut c_void)>,
) -> c_int {
    ::nvx::trace!("pthread_key_create(key_ptr={:p}, destructor={:?})", key_ptr, destructor);

    // Check if storage location for the key is valid.
    if key_ptr.is_null() {
        ::nvx::error!("pthread_key_create(): invalid storage location for thread key");
        return ErrorCode::InvalidArgument.into_errno();
    }

    // Check if destructor is not null.
    if destructor.is_some() {
        ::nvx::error!("pthread_key_create(): destructors are not supported");
        return ErrorCode::OperationNotSupported.into_errno();
    }

    // Create key.
    match syscall::pthread_key_create() {
        Some(key) => {
            *key_ptr = key;
            0
        },
        None => {
            ::nvx::error!("pthread_key_create(): failed to create key");
            ErrorCode::OutOfMemory.into_errno()
        },
    }
}

//==================================================================================================
// pthread_key_delete()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_key_delete(key: pthread_key_t) -> c_int {
    ::nvx::trace!("pthread_key_delete(): key={}", key);

    match syscall::pthread_key_delete(key) {
        Ok(()) => 0,
        Err(error) => error.code.into_errno(),
    }
}

//==================================================================================================
// pthread_getspecific()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_getspecific(key: pthread_key_t) -> *mut c_void {
    ::nvx::trace!("pthread_getspecific(): key={}", key);

    match syscall::pthread_getspecific(key) {
        Ok(value) => value.into(),
        Err(_error) => core::ptr::null_mut(),
    }
}

//==================================================================================================
// pthread_setspecific()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_setspecific(key: pthread_key_t, value: *const c_void) -> c_int {
    ::nvx::trace!("pthread_setspecific(): key={}, value={:p}", key, value);

    match syscall::pthread_setspecific(key, value.into()) {
        Ok(()) => 0,
        Err(error) => error.code.into_errno(),
    }
}
