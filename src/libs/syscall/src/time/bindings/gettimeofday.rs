// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sysapi::{
    ffi::{
        c_int,
        c_long,
        c_void,
    },
    sys_time::{
        timeval,
        timezone,
    },
    time::{
        clock_ids::CLOCK_REALTIME,
        timespec,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Constants
//==================================================================================================

/// Number of nanoseconds in a microsecond.
const NANOSECONDS_PER_MICROSECOND: c_long = 1_000;

/// Time zone of the system clock.
///
/// The system clock is not adjusted for any time zone, and no time zone can be set, because
/// `settimeofday()` always fails. The time zone is therefore Coordinated Universal Time (UTC): zero
/// minutes west of Greenwich, with no daylight saving time correction.
const SYSTEM_TIMEZONE: timezone = timezone {
    tz_minuteswest: 0,
    tz_dsttime: 0,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the current time, expressed as seconds and microseconds since the Epoch, and the time
/// zone of the system clock.
///
/// The time is read from the `CLOCK_REALTIME` clock and truncated to microsecond resolution, so it
/// is consistent with the time that `clock_gettime()` reports for that clock. The time zone is
/// always Coordinated Universal Time (UTC).
///
/// # Parameters
///
/// - `tv`: Storage location for the current time. If null, the time is not retrieved.
/// - `tz`: Storage location for the time zone, as a `struct timezone`. If null, the time zone is
///   not retrieved. This argument is obsolete and should be null.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, `-1` is returned, `errno` is set to
/// indicate the error, and the storage locations pointed to by `tv` and `tz` are left unchanged.
///
/// # Safety
///
/// This function is unsafe because it writes through raw pointers and accesses the global `errno`
/// location.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `tv` is null or points to a valid, writable `struct timeval`.
/// - `tz` is null or is valid for writes of a `struct timezone`.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int {
    // Read the time before storing any result, so that no output changes if reading fails.
    if !tv.is_null() {
        let mut now: timespec = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        if let Err(error) = crate::time::clock_gettime(CLOCK_REALTIME, &mut Some(&mut now)) {
            ::syslog::warn!("gettimeofday(): failed (tv={tv:p}, tz={tz:p}, error={error:?})");
            *__errno_location() = error.code.get();
            return -1;
        }

        // `clock_gettime()` reports nanoseconds below one second, so the microseconds are below one
        // second too. Truncating them, rather than rounding, keeps the time from running ahead of
        // `CLOCK_REALTIME`.
        *tv = timeval {
            tv_sec: now.tv_sec,
            tv_usec: now.tv_nsec / NANOSECONDS_PER_MICROSECOND,
        };
    }

    if !tz.is_null() {
        // `tz` is untyped, so it is not guaranteed to be aligned for `struct timezone`.
        tz.cast::<timezone>().write_unaligned(SYSTEM_TIMEZONE);
    }

    0
}
