// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::InterruptNumber,
    pm::ProcessManager,
};
use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
};

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

    /// Increments the number of ticks and returns the minor tick.
    fn increment(&self) -> u32 {
        // Safely increment `TIMER_TICKS`, assuming single-writer.
        let minor: u32 = self.minor.load(Ordering::SeqCst);
        let new_minor: u32 = minor.wrapping_add(1);
        self.minor.store(new_minor, Ordering::SeqCst);

        // Check if the minor tick overflowed.
        if new_minor == 0 {
            // Safely increment `TIMER_TICKS_MAJOR`, assuming single-writer.
            let major: u32 = self.major.load(Ordering::SeqCst);
            self.major.store(major.wrapping_add(1), Ordering::SeqCst);
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
    let timer_ticks: usize = TIMER_TICKS.increment() as usize;

    // Determine if a context switch is required. The kernel's running state is checked first to
    // prevent reentrant calls to the scheduler, which could lead to undefined behavior.
    if !ProcessManager::is_kernel_running() && timer_ticks % config::kernel::SCHEDULER_FREQ == 0 {
        if let Err(error) = ProcessManager::switch() {
            error!("context switch failed: {:?}", error);
        }
    }
}
