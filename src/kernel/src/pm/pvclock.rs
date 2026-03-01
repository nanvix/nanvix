// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! KVM paravirtualized clock — time conversion layer.
//!
//! Combines the platform-specific monotonic and boot-time readings from the
//! microvm pvclock helpers to produce wall-clock `SystemTime` values.
//!
//! Reference: <https://docs.kernel.org/virt/kvm/x86/msr.html#pvclock>

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::platform::pvclock::{
    boot_time_ns,
    monotonic_time_ns,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of nanoseconds per second.
const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the current wall-clock time in nanoseconds since the Unix epoch using the
/// paravirtualized clock.
///
/// Wall-clock time = boot_time_ns + monotonic_time_ns
///
/// # Returns
///
/// - `Some(ns)`: UTC nanoseconds since 1970-01-01 00:00:00.
/// - `None`: The pvclock is not initialized.
///
pub fn wall_clock_time_ns() -> Option<u64> {
    let mono_ns: u64 = monotonic_time_ns()?;
    let boot_ns: u64 = boot_time_ns();
    Some(boot_ns.wrapping_add(mono_ns))
}

///
/// # Description
///
/// Returns the current system time using the paravirtualized clock.
///
/// # Returns
///
/// - `Some(SystemTime)`: Current UTC wall-clock time.
/// - `None`: The pvclock is not initialized.
///
pub fn now() -> Option<::sys::time::SystemTime> {
    let total_ns: u64 = wall_clock_time_ns()?;
    let seconds: u64 = total_ns / NANOSECONDS_PER_SECOND;
    let nanos_remainder: u64 = total_ns % NANOSECONDS_PER_SECOND;

    // NOTE: `nanos_remainder` is always < 1_000_000_000 which fits in u32.
    let nanoseconds: u32 = match u32::try_from(nanos_remainder) {
        Ok(v) => v,
        Err(_) => {
            // This branch is unreachable because `n % 1_000_000_000 < u32::MAX`.
            return None;
        },
    };

    ::sys::time::SystemTime::new(seconds, nanoseconds)
}
