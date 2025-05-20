// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::syscall::{
    ffi::{
        c_int,
        c_void,
    },
    pthread,
    sys::types::pthread_key_t,
};

//==================================================================================================
//pthread_key_create()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_key_create(
    key_ptr: *mut pthread_key_t,
    destructor: Option<extern "C" fn(*mut c_void)>,
) -> c_int {
    // Check if storage location for the key is valid.
    if key_ptr.is_null() {
        ::syslog::error!("pthread_key_create(): invalid storage location for thread key");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if destructor is not null.
    if destructor.is_some() {
        ::syslog::error!("pthread_key_create(): destructors are not supported");
        return ErrorCode::OperationNotSupported.get();
    }

    // Create key.
    match pthread::pthread_key_create() {
        Some(key) => {
            *key_ptr = key;
            0
        },
        None => {
            ::syslog::error!("pthread_key_create(): failed to create key");
            ErrorCode::OutOfMemory.get()
        },
    }
}

//==================================================================================================
// pthread_key_delete()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_key_delete(key: pthread_key_t) -> c_int {
    match pthread::pthread_key_delete(key) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}

//==================================================================================================
// pthread_getspecific()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_getspecific(key: pthread_key_t) -> *mut c_void {
    match pthread::pthread_getspecific(key) {
        Ok(value) => value.into(),
        Err(_error) => core::ptr::null_mut(),
    }
}

//==================================================================================================
// pthread_setspecific()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_setspecific(key: pthread_key_t, value: *const c_void) -> c_int {
    match pthread::pthread_setspecific(key, value.into()) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}
