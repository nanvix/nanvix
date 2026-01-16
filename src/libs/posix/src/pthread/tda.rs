// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_key_t,
};
use ::syscall::pthread;
use ::syslog::trace_libcall;

//==================================================================================================
// pthread_key_delete()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_key_delete(key: pthread_key_t) -> c_int {
    match pthread::pthread_key_delete(key) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}
