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
        ::syslog::warn!("pthread_key_create(): invalid storage location for thread key");
        return ErrorCode::InvalidArgument.get();
    }

    // Destructors are not yet supported (would require per-thread
    // cleanup on thread exit, which Nanvix's single-threaded
    // user-space model has no use for).  Rather than refuse the
    // call -- which breaks libraries like OpenSSL that always pass
    // a non-null destructor as a defensive measure -- we accept
    // the call, log a one-line warning, and silently drop the
    // destructor.  Worst case is a per-thread resource leak at
    // thread exit, which is benign in a process-lifetime model.
    if destructor.is_some() {
        ::syslog::warn!("pthread_key_create(): destructor ignored (not yet supported)");
    }

    // Create key.
    match crate::pthread::pthread_key_create() {
        Some(key) => {
            *key_ptr = key;
            0
        },
        None => {
            ::syslog::warn!("pthread_key_create(): failed to create key");
            ErrorCode::OutOfMemory.get()
        },
    }
}
