// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![forbid(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::char_lit_as_u8,
    clippy::fn_to_numeric_cast,
    clippy::fn_to_numeric_cast_with_truncation,
    clippy::ptr_as_ptr,
    clippy::unnecessary_cast,
    invalid_reference_casting
)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::InterruptNumber,
    pm::{
        ProcessManager,
        ORDER,
    },
};
use ::core::sync::atomic::AtomicU32;
use ::sys::time::{
    SystemTime,
    NANOSECONDS_PER_SECOND,
};

#[cfg(feature = "microvm")]
#[path = "pvclock.rs"]
mod pvclock;

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Counts the number of times in which the [`timer_handler`] was called.
///
static TIMER_TICKS: TimerTicks = TimerTicks::new();

//==================================================================================================
// TimerTicks
//==================================================================================================

/// A counter for the number of timer ticks.
struct TimerTicks {
    minor: AtomicU32,
    major: AtomicU32,
}

impl TimerTicks {
    const fn new() -> Self {
        Self {
            minor: AtomicU32::new(0),
            major: AtomicU32::new(0),
        }
    }

    /// Returns the number of ticks since the system started.
    fn get(&self) -> (u32, u32) {
        (self.major.load(ORDER), self.minor.load(ORDER))
    }

    /// Increments the number of ticks and returns the minor tick.
    fn increment(&self) -> u32 {
        // Safely increment `TIMER_TICKS`, assuming single-writer.
        let minor: u32 = self.minor.load(ORDER);
        let new_minor: u32 = minor.wrapping_add(1);
        self.minor.store(new_minor, ORDER);

        // Check if the minor tick overflowed.
        if new_minor == 0 {
            // Safely increment `TIMER_TICKS_MAJOR`, assuming single-writer.
            let major: u32 = self.major.load(ORDER);
            self.major.store(major.wrapping_add(1), ORDER);
        }

        new_minor
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Handles a timer interrupt.
///
/// # Parameters
///
/// - `intnum`: The number of the interrupt that triggered the handler.
///
/// # Safety
///
/// This function is unsafe because:
/// - It may perform a context switch, suspending the current thread.
/// - It modifies global variables.
///
pub unsafe fn timer_handler(_intnum: InterruptNumber) {
    TIMER_TICKS.increment();

    // Check if a pause has been requested.
    #[cfg(feature = "microvm")]
    if core::ptr::read_volatile(
        ::config::microvm::DEFAULT_MICROVM_CTRL_PAUSE_REQUESTED as *const u32,
    ) == ::config::microvm::PAUSE_REQUEST
    {
        // Cause a VM exit.
        ::arch::io::out32(
            ::config::microvm::DEFAULT_VMM_PORT,
            (::config::microvm::DEFAULT_VMM_PAUSE_CMD as u32) << 16,
        )
    }

    // Determine if a context switch is required. The kernel's running state is checked first to
    // prevent reentrant calls to the scheduler, which could lead to undefined behavior.
    if !ProcessManager::is_kernel_running() {
        if let Err(error) = ProcessManager::tick() {
            error!("context switch failed: {:?}", error);
        }
    }
}

///
/// # Description
///
/// Returns the number of timer ticks since the system started.
///
/// # Returns
///
/// The number of timer ticks since the system started.
pub fn ticks() -> u64 {
    let (major_ticks, minor_ticks): (u32, u32) = TIMER_TICKS.get();
    ((major_ticks as u64) << 32) + (minor_ticks as u64)
}

///
/// # Description
///
/// Returns the current kernel time.
///
/// When the KVM paravirtualized clock is available (microvm feature), this function reads the
/// pvclock page and TSC to compute a wall-clock timestamp (UTC) without a VM exit.
///
/// If pvclock is not initialized or not available, this function falls back to PIT-based tick
/// counting derived from `TIMER_TICKS`. In that case, the returned value represents a
/// monotonic time since system boot (uptime) and is **not** guaranteed to be an epoch-based
/// wall-clock/UTC timestamp.
///
pub fn now() -> SystemTime {
    // Try pvclock first when running on a microvm.
    #[cfg(feature = "microvm")]
    if let Some(time) = pvclock::now() {
        return time;
    }

    // Fallback: PIT-based tick counting.
    #[cfg(all(feature = "pit", any(target_arch = "x86", target_arch = "x86_64")))]
    let timer_freq: u32 = crate::hal::platform::pit::get_timer_frequency();
    #[cfg(target_arch = "aarch64")]
    let timer_freq: u32 = ::config::kernel::TIMER_FREQ;
    #[cfg(all(not(feature = "pit"), any(target_arch = "x86", target_arch = "x86_64")))]
    let timer_freq: u32 = 1;

    let (major_ticks, minor_ticks): (u32, u32) = TIMER_TICKS.get();
    let seconds: u64 = (((major_ticks as u64) << 32) + (minor_ticks as u64)) / (timer_freq as u64);
    let nanoseconds: u32 = (minor_ticks % timer_freq) * (NANOSECONDS_PER_SECOND / timer_freq);

    match SystemTime::new(seconds, nanoseconds) {
        Some(time) => time,
        None => {
            // SAFETY: This should not happen because `ticks` should be always in a valid range of `SystemTime`.
            unreachable!(
                "now(): failed to get system time (major_ticks={major_ticks:?}, \
                 minor_ticks={minor_ticks:?}, timer_freq={timer_freq:?})"
            )
        },
    }
}
