// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_uint,
    },
    sys_types::sem_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the unnamed semaphore referred to by `sem` and sets its initial value to `value`.
///
/// # Parameters
///
/// - `sem`: Semaphore to initialize.
/// - `pshared`: Whether the semaphore is shared between processes.
/// - `value`: Initial value of the semaphore.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, `-1` is returned and `errno` is set to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `sem` points to a valid `sem_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn sem_init(sem: *mut sem_t, pshared: c_int, value: c_uint) -> c_int {
    // Check if `sem` is not valid.
    if sem.is_null() {
        ::syslog::warn!("sem_init(): invalid semaphore pointer");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Process-shared semaphores are not supported; only process-private semantics are provided.
    if pshared != 0 {
        ::syslog::warn!("sem_init(): process-shared semaphores are not supported");
        *__errno_location() = ErrorCode::OperationNotSupported.get();
        return -1;
    }

    match crate::pthread::sem_init(&mut *sem, value) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::warn!("sem_init(): {error:?}");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
