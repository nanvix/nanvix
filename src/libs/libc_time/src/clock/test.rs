// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
