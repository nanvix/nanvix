// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pthread::syscall;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sched::sched_param,
    sys_types::pthread_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the scheduling policy and scheduling parameters of a thread.
///
/// # Parameters
///
/// - `thread`: Thread whose scheduling policy and parameters will be retrieved.
/// - `policy`: Storage location for the scheduling policy.
/// - `param`: Storage location for the scheduling parameters.
///
/// # Return Value
///
/// On success, this function stores the scheduling policy and the scheduling parameters of
/// `thread` in `policy` and `param`, respectively, and returns zero. Otherwise, it returns a
/// non-zero error code indicating the reason for the failure and leaves `policy` and `param`
/// unchanged.
///
/// # Errors
///
/// The following errors can be returned by this function:
///
/// - [`ErrorCode::InvalidArgument`] if `policy` points to an invalid address.
/// - [`ErrorCode::InvalidArgument`] if `policy` points to a misaligned address.
/// - [`ErrorCode::InvalidArgument`] if `param` points to an invalid address.
/// - [`ErrorCode::InvalidArgument`] if `param` points to a misaligned address.
/// - [`ErrorCode::NoSuchProcess`] if `thread` cannot identify a thread.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `policy` points to a valid `c_int` variable.
/// - `param` points to a valid `sched_param` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_getschedparam(
    thread: pthread_t,
    policy: *mut c_int,
    param: *mut sched_param,
) -> c_int {
    // Check if `policy` points to an invalid address.
    if policy.is_null() {
        ::syslog::warn!(
            "pthread_getschedparam(): invalid pointer to scheduling policy (thread={thread:?}, \
             policy={policy:p}, param={param:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `policy` points to a misaligned address.
    if !(policy as usize).is_multiple_of(core::mem::align_of::<c_int>()) {
        ::syslog::warn!(
            "pthread_getschedparam(): misaligned pointer to scheduling policy (thread={thread:?}, \
             policy={policy:p}, param={param:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `param` points to an invalid address.
    if param.is_null() {
        ::syslog::warn!(
            "pthread_getschedparam(): invalid pointer to scheduling parameters \
             (thread={thread:?}, policy={policy:p}, param={param:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `param` points to a misaligned address.
    if !(param as usize).is_multiple_of(core::mem::align_of::<sched_param>()) {
        ::syslog::warn!(
            "pthread_getschedparam(): misaligned pointer to scheduling parameters \
             (thread={thread:?}, policy={policy:p}, param={param:p})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Attempt to get the scheduling policy and parameters and check for errors.
    match syscall::pthread_getschedparam(thread) {
        Ok((thread_policy, thread_param)) => {
            // Write through the raw pointers instead of references, because the caller may pass
            // overlapping storage locations.
            *policy = thread_policy;
            *param = thread_param;
            ::syslog::trace!("pthread_getschedparam(): success (thread={thread:?})");
            0
        },
        Err(error) => {
            ::syslog::warn!("pthread_getschedparam(): {error:?} (thread={thread:?})");
            error.code.get()
        },
    }
}
