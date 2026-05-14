// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::KernelThread;
use ::core::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use ::sys::{
    error::Error,
    kcall::sched::__kcall_sched_yield,
};
use ::sysapi::unistd::STDOUT_FILENO;
use ::syscall::unistd;

//==================================================================================================
// Global State
//==================================================================================================

static WORKERS_FINISHED: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Executes detached-thread tests.
pub fn run() -> Result<(), Error> {
    test_detach_and_exit_stress()?;
    Ok(())
}

/// Stress-tests detached thread exit to trigger use-after-free.
///
/// Spawns multiple detached threads that exit immediately. Each detached-thread exit frees a
/// `ContextInformation` slab slot (128 bytes); if the bug is present, the subsequent context
/// switch writes CPU state through a dangling pointer into that freed slot.
///
/// NOTE: This test exercises the exact code path that was buggy but cannot deterministically
/// trigger a crash in isolation. The original bug manifested under heavy allocation pressure
/// (e.g., CPython's thread-heavy workload) where the freed slab slot was reused by a different
/// object type (VecDeque nodes, BTreeMap entries) before the context switch wrote to it. In this
/// simpler test, the freed slot is typically reused by the next iteration's `ContextInformation`
/// allocation and re-initialized, masking the corruption. The test remains valuable as a
/// regression test that exercises the spawn→detach→exit→continue pattern and will catch any
/// future breakage of that path.
fn test_detach_and_exit_stress() -> Result<(), Error> {
    const ITERATIONS: usize = 50;

    WORKERS_FINISHED.store(0, Ordering::Relaxed);

    for _ in 0..ITERATIONS {
        let handle: KernelThread = KernelThread::spawn(worker_fast, 0)?;
        handle.detach()?;

        // Yield to let the detached worker run and exit. The context switch back to us is where
        // the UAF would corrupt memory.
        __kcall_sched_yield()?;
        __kcall_sched_yield()?;
    }

    // Wait for all workers to finish.
    for _ in 0..2000u32 {
        if WORKERS_FINISHED.load(Ordering::Acquire) >= ITERATIONS {
            break;
        }
        __kcall_sched_yield()?;
    }

    assert!(
        WORKERS_FINISHED.load(Ordering::Acquire) >= ITERATIONS,
        "not all detached workers finished"
    );

    // Exercise the event manager via a write syscall. If any scheduler data structure was
    // corrupted by the UAF, this will likely panic.
    let msg: &[u8] = b"";
    unistd::write(STDOUT_FILENO, msg)?;

    Ok(())
}

extern "C" fn worker_fast(_arg: usize) -> usize {
    WORKERS_FINISHED.fetch_add(1, Ordering::Release);
    0
}
