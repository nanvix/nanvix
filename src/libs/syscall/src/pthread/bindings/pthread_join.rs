// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::pthread_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Waits for a thread to terminate.
///
/// # Parameters
///
/// - `thread`: Thread identifier.
/// - `retval_ptr`: Pointer to the location where the return value of the thread will be stored.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is call to safe this function if the following conditions are met:
///
/// - If `retval_ptr` is not null, it points to a valid pointer.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_join(thread: pthread_t, retval_ptr: *mut *mut c_void) -> c_int {
    match crate::pthread::pthread_join(thread) {
        Ok(retval) => {
            ::syslog::trace!("pthread_join(): retval={:#x?}", retval);
            if !retval_ptr.is_null() {
                *retval_ptr = retval as *mut c_void;
            }
            0
        },
        Err(error) => error.code.get(),
    }
}
