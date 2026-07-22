// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_int,
        c_long,
    },
    sys_types::{
        clock_t,
        clockid_t,
    },
    time::{
        clock_ids::CLOCK_MONOTONIC,
        timespec,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of clock ticks per second.
pub const CLOCKS_PER_SEC: clock_t = 1_000_000;

//==================================================================================================
// External Functions
//==================================================================================================

extern "C" {
    fn clock_gettime(clock_id: clockid_t, tp: *mut timespec) -> c_int;
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Converts a monotonic-clock `timespec` into clock ticks of `CLOCKS_PER_SEC` (microseconds).
///
/// Sub-microsecond nanosecond resolution is truncated, matching the microsecond tick rate that
/// `CLOCKS_PER_SEC` advertises. Returns `None` if the conversion overflows `clock_t`.
fn ticks_from_timespec(sec: clock_t, nsec: c_long) -> Option<clock_t> {
    let nsec: clock_t = clock_t::from(nsec);
    sec.checked_mul(CLOCKS_PER_SEC)?.checked_add(nsec / 1_000)
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns an approximation of processor time used by the program, expressed in clock ticks of
/// `CLOCKS_PER_SEC` (one microsecond). Nanvix approximates this with the monotonic clock.
///
/// # Returns
///
/// The elapsed time in microseconds, or `(clock_t)-1` if it is unavailable.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/clock.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn clock() -> clock_t {
    let mut ts: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: ts is a valid, writable timespec.
    let rc: c_int = unsafe { clock_gettime(CLOCK_MONOTONIC, &mut ts) };
    if rc != 0 {
        return -1;
    }
    ticks_from_timespec(ts.tv_sec, ts.tv_nsec).unwrap_or(-1)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::ticks_from_timespec;
    use ::sysapi::sys_types::clock_t;

    #[test]
    fn test_ticks_whole_seconds() {
        // 2 seconds expressed in microsecond ticks.
        assert_eq!(ticks_from_timespec(2 as clock_t, 0), Some(2_000_000));
    }

    #[test]
    fn test_ticks_nanoseconds_to_micros() {
        // 1_500_000 ns is 1_500 microseconds.
        assert_eq!(ticks_from_timespec(0 as clock_t, 1_500_000), Some(1_500));
    }

    #[test]
    fn test_ticks_combined() {
        // 1 s plus 1_000 ns (1 microsecond).
        assert_eq!(ticks_from_timespec(1 as clock_t, 1_000), Some(1_000_001));
    }

    #[test]
    fn test_ticks_sub_microsecond_truncated() {
        // Sub-microsecond resolution is discarded.
        assert_eq!(ticks_from_timespec(0 as clock_t, 999), Some(0));
    }

    #[test]
    fn test_ticks_overflow_returns_none() {
        // A tick count that overflows `clock_t` is reported as an error.
        assert_eq!(ticks_from_timespec(clock_t::MAX, 0), None);
    }
}
