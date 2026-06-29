// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::time::Duration;
use ::sys::{
    error::ErrorCode,
    kcall::pm,
    time::SystemTime,
};
use ::sysapi::{
    ffi::c_uint,
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
/// Suspends the calling thread until either the number of real-time seconds specified by `seconds`
/// has elapsed, or a signal whose action is to invoke a handler or to terminate the process is
/// delivered to the calling thread.
///
/// # Parameters
///
/// - `seconds`: The number of seconds to sleep for.
///
/// # Returns
///
/// Returns `0` if the requested time has elapsed. If the sleep was interrupted by a signal, returns
/// the unslept amount (the requested time minus the time actually slept) in seconds, rounded up to
/// whole seconds so that a caller re-sleeping the returned value never sleeps for less than the
/// originally requested time.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub extern "C" fn sleep(seconds: c_uint) -> c_uint {
    // A zero-second request completes immediately, with no kernel round-trip.
    if seconds == 0 {
        return 0;
    }

    let duration: Duration = Duration::from_secs(seconds as u64);
    let req: timespec = timespec {
        tv_sec: seconds as time_t,
        tv_nsec: 0,
    };

    // Record the start time so that the unslept interval can be reported if a signal interrupts the
    // sleep before the full duration elapses.
    let mut start: SystemTime = SystemTime::default();
    let measured: bool = pm::__kcall_gettime(&mut start).is_ok();

    // Suspend the calling thread for the requested interval.
    let mut rem: Option<&mut timespec> = None;
    match crate::time::nanosleep(&req, &mut rem) {
        // The full interval elapsed.
        Ok(()) => return 0,
        // A signal interrupted the sleep; fall through to report the unslept amount.
        Err(e) if e.code == ErrorCode::Interrupted => {},
        // Any other failure means that nothing was slept, so report the full interval as unslept.
        Err(e) => {
            ::syslog::warn!("sleep(): nanosleep() failed ({:?})", e.code);
            return seconds;
        },
    }

    // A signal interrupted the sleep. Report the unslept whole seconds when the elapsed time could
    // be measured; otherwise report the full interval as unslept.
    if !measured {
        return seconds;
    }

    let mut now: SystemTime = SystemTime::default();
    if pm::__kcall_gettime(&mut now).is_err() {
        return seconds;
    }

    let elapsed: Duration = now.checked_sub(&start).unwrap_or(Duration::ZERO);

    // Round the unslept interval up to whole seconds so that a caller re-sleeping the returned
    // amount never sleeps for less than the originally requested time.
    let remaining: Duration = duration.saturating_sub(elapsed);
    let unslept: u64 = remaining.as_secs() + (remaining.subsec_nanos() > 0) as u64;
    unslept as c_uint
}
