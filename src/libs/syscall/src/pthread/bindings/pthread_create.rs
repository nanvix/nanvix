// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        pthread_attr_t,
        pthread_t,
    },
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new thread.
///
/// # Parameters
///
/// - `thread`: Thread identifier.
/// - `attr`: Thread attributes.
/// - `start_routine`: Thread function.
/// - `arg`: Argument passed to the thread function.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is call to safe this function if the following conditions are met:
///
/// - `thread` points to a valid `pthread_t` structure.
/// - If `attr` is not null, it points to a valid `pthread_attr_t` structure.
/// - `start_routine` is a valid function pointer.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_create(
    thread: *mut pthread_t,
    attr: *const pthread_attr_t,
    start_routine: extern "C" fn(usize) -> usize,
    arg: usize,
) -> c_int {
    // Check if `thread` is not valid.
    if thread.is_null() {
        ::syslog::error!("pthread_create(): invalid thread pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Cast `thread` to a mutable reference.
    let thread: &mut pthread_t = &mut *thread;

    // Check if we should use default attributes.
    if attr.is_null() {
        // TODO: use default attributes.
    } else {
        ::syslog::warn!("pthread_create(): attributes are not supported, ignoring");
    }

    match crate::pthread::pthread_create(start_routine, arg) {
        Ok(tid) => {
            *thread = tid;
            0
        },
        Err(error) => error.code.get(),
    }
}
