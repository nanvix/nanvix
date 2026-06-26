// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::{
    error::ErrorCode,
    time::NANOSECONDS_PER_SECOND,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_long,
    },
    sys_types::clockid_t,
    time::{
        clock_ids::{
            CLOCK_MONOTONIC,
            CLOCK_PROCESS_CPUTIME_ID,
            CLOCK_REALTIME,
            CLOCK_THREAD_CPUTIME_ID,
        },
        timespec,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the value of the clock identified by `clock_id`. Setting a clock requires privileges that
/// Nanvix does not grant to user processes, so valid requests fail with `EPERM`.
///
/// # Parameters
///
/// - `clock_id`: The identifier of the clock to be set.
/// - `tp`: The structure holding the time the clock should be set to.
///
/// # Returns
///
/// Always returns `-1` with `errno` set to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it accesses the global `errno` location.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clock_settime(clock_id: clockid_t, tp: *const timespec) -> c_int {
    match clock_id {
        CLOCK_REALTIME | CLOCK_MONOTONIC | CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => {},
        _ => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    }

    if tp.is_null() {
        *__errno_location() = ErrorCode::BadAddress.get();
        return -1;
    }

    let tv_nsec: c_long = (*tp).tv_nsec;
    if tv_nsec < 0 || tv_nsec >= NANOSECONDS_PER_SECOND as c_long {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Adjusting the system clock is a privileged operation that Nanvix does not expose.
    *__errno_location() = ErrorCode::OperationNotPermitted.get();
    -1
}
