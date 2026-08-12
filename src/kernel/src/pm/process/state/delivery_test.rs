// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::ProcessState;
use crate::{
    mm::{
        VirtMemoryManager,
        Vmem,
    },
    pm::ProcessManager,
};
use ::sys::pm::ProcessIdentifier;

//==================================================================================================
// Test Helpers
//==================================================================================================

/// Creates a process state backed by a fresh virtual memory space.
fn make_process_state(pid: ProcessIdentifier) -> Option<ProcessState> {
    // SAFETY: the process and virtual memory managers are initialized before these in-kernel tests
    // run, and access is synchronized during single-threaded kernel startup.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
    let vmem: Vmem = match mm.new_vmem(pm.current_vmem()) {
        Ok(vmem) => vmem,
        Err(error) => {
            error!("failed to create cursor-test address space (error={error:?})");
            return None;
        },
    };
    Some(ProcessState::new(pid, ProcessIdentifier::KERNEL, None, vmem))
}

//==================================================================================================
// Tests
//==================================================================================================

/// Verifies that changing one process's delivery cursor cannot perturb another process's cursor.
fn test_delivery_cursor_is_process_local() -> bool {
    let mut first: ProcessState = match make_process_state(ProcessIdentifier::from(1000)) {
        Some(state) => state,
        None => return false,
    };
    let mut second: ProcessState = match make_process_state(ProcessIdentifier::from(1001)) {
        Some(state) => state,
        None => return false,
    };

    first.set_delivery_cursor(2);
    if first.delivery_cursor() != 2 || second.delivery_cursor() != 0 {
        error!("first process's cursor update perturbed the second process");
        return false;
    }

    second.set_delivery_cursor(1);
    if first.delivery_cursor() != 2 || second.delivery_cursor() != 1 {
        error!("second process's cursor update perturbed the first process");
        return false;
    }

    true
}

/// Runs delivery-state in-kernel tests.
pub(super) fn test() -> bool {
    run_test!(test_delivery_cursor_is_process_local)
}
