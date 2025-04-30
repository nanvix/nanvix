// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::InterruptNumber,
    pm::ProcessManager,
};

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
/// - It may modify global variables.
///
pub unsafe fn timer_handler(_intnum: InterruptNumber) {
    /// Counts the number of times in which the [`timer_handler`] was called.
    static mut TIMER_TICKS: usize = 0;

    TIMER_TICKS = TIMER_TICKS.wrapping_add(1);

    if TIMER_TICKS % config::kernel::SCHEDULER_FREQ == 0 {
        if let Err(error) = ProcessManager::switch() {
            error!("context switch failed: {:?}", error);
        }
    }
}
