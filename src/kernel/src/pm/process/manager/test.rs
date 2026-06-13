// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::ProcessManager;
use ::config::kernel::SCHEDULER_FREQ;
use ::sys::pm::ProcessIdentifier;

//==================================================================================================
// Tests
//==================================================================================================

///
/// # Description
///
/// Verifies that an intra-process context switch that follows the exhaustion of the outgoing
/// thread's quantum resets the quantum for the incoming thread.
///
/// This is a regression test for the quantum-inheritance starvation bug (issue #1695): when a
/// thread is preempted because its quantum reached zero and the scheduler selects another thread of
/// the same process, the incoming thread must not inherit the exhausted quantum. Otherwise it is
/// immediately preempted on the next tick and is permanently starved by its sibling threads.
///
fn test_intra_process_switch_resets_exhausted_quantum() -> bool {
    let pid: ProcessIdentifier = ProcessIdentifier::from(1);

    // The outgoing thread exhausted its quantum (remaining == 0) and the scheduler selected another
    // thread of the same process. The incoming thread must start with a fresh quantum.
    let quantum: usize = ProcessManager::next_thread_quantum(pid, pid, 0);
    if quantum != SCHEDULER_FREQ {
        error!(
            "intra-process switch after quantum exhaustion did not reset the quantum (got {}, \
             expected {})",
            quantum, SCHEDULER_FREQ
        );
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that an intra-process context switch that follows a voluntary yield (the outgoing
/// thread still had quantum left) preserves the remaining quantum for the incoming thread.
///
/// This ensures the fix for issue #1695 does not regress into unconditionally resetting the quantum
/// on every thread switch (the naive fix), which would let a process whose threads frequently yield
/// accumulate more than its fair share of CPU time and starve other processes.
///
fn test_intra_process_switch_preserves_remaining_quantum() -> bool {
    let pid: ProcessIdentifier = ProcessIdentifier::from(1);

    // Pick a remaining quantum that is strictly between zero and a full quantum.
    let remaining: usize = SCHEDULER_FREQ / 2;
    let quantum: usize = ProcessManager::next_thread_quantum(pid, pid, remaining);
    if quantum != remaining {
        error!(
            "intra-process voluntary yield did not preserve the remaining quantum (got {}, \
             expected {})",
            quantum, remaining
        );
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that a cross-process context switch always starts the incoming process with a fresh
/// quantum, regardless of how much quantum the outgoing thread had left.
///
fn test_cross_process_switch_resets_quantum() -> bool {
    let previous_pid: ProcessIdentifier = ProcessIdentifier::from(1);
    let next_pid: ProcessIdentifier = ProcessIdentifier::from(2);

    // A cross-process switch must reset the quantum whether or not the outgoing thread had quantum
    // left.
    for remaining in [0, SCHEDULER_FREQ / 2, SCHEDULER_FREQ] {
        let quantum: usize = ProcessManager::next_thread_quantum(next_pid, previous_pid, remaining);
        if quantum != SCHEDULER_FREQ {
            error!(
                "cross-process switch did not reset the quantum (remaining={}, got {}, expected \
                 {})",
                remaining, quantum, SCHEDULER_FREQ
            );
            return false;
        }
    }

    true
}

//==================================================================================================
// Test Runner
//==================================================================================================

/// Runs all in-kernel unit tests for the process manager module.
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_intra_process_switch_resets_exhausted_quantum);
    passed &= run_test!(test_intra_process_switch_preserves_remaining_quantum);
    passed &= run_test!(test_cross_process_switch_resets_quantum);
    passed
}
