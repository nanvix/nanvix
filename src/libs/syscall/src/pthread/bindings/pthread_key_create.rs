// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::pthread_key_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
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
    match crate::pthread::pthread_key_create() {
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
