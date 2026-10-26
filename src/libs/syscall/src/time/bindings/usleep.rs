// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::time::Duration;
use ::sysapi::{
    ffi::{
        c_int,
        c_long,
        c_uint,
    },
    sys_types::time_t,
    time::timespec,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Suspends the calling thread until either the number of real-time microseconds specified by
/// `usec` has elapsed, or a signal whose action is to invoke a handler or to terminate the process
/// is delivered to the calling thread. The suspension may be longer than requested, due to the
/// granularity of the system timer and the scheduling of other activity by the system.
///
/// POSIX allows, but does not require, rejecting intervals of one second or more with `EINVAL`.
/// Such intervals are honored instead, as other common C libraries do, so that programs relying on
/// this behavior keep working.
///
/// # Parameters
///
/// - `usec`: The number of microseconds to sleep for. A value of `0` has no effect.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, `-1` is returned and `errno` is set
/// to indicate the error.
///
/// # Errors
///
/// - `EINTR`: A signal interrupted the sleep.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub extern "C" fn usleep(usec: c_uint) -> c_int {
    // A zero-microsecond request has no effect, so it completes with no kernel round-trip.
    if usec == 0 {
        return 0;
    }

    // Both casts are lossless: `c_uint::MAX` microseconds is far below `time_t::MAX` seconds, and
    // the sub-second part is always below one billion nanoseconds, which fits in `c_long`.
    let duration: Duration = Duration::from_micros(u64::from(usec));
    let req: timespec = timespec {
        tv_sec: duration.as_secs() as time_t,
        tv_nsec: duration.subsec_nanos() as c_long,
    };

    // Suspend the calling thread for the requested interval.
    let mut rem: Option<&mut timespec> = None;
    match crate::time::nanosleep(&req, &mut rem) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::warn!("usleep(): failed (usec={}, error={:?})", usec, error);
            // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
            unsafe {
                *__errno_location() = error.code.get();
            }
            -1
        },
    }
}
