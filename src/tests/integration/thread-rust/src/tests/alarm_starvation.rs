// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::{
    KernelThread,
    deadline_from_now,
    monotonic_now,
};
use ::core::{
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        pm::__kcall_sleep,
        sched::__kcall_sched_yield,
    },
    time::SystemTime,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Timed-sleep duration for the sleeper thread. Kept short so the test completes quickly once the
/// scheduler services per-thread alarms correctly.
const SLEEPER_TIMEOUT: Duration = Duration::from_millis(100);

/// Wall-clock budget for the CPU-bound sibling. Chosen an order of magnitude larger than
/// `SLEEPER_TIMEOUT`: when the sleeper's alarm is serviced correctly, the sibling is still running
/// when the sleeper wakes. The sibling self-limits to this budget so that the test still terminates
/// on a buggy kernel, where the sleeper cannot wake until the sibling stops.
const BUSY_BUDGET: Duration = Duration::from_millis(1000);

/// Value returned by the sibling worker on success.
const WORKER_OK: usize = 0xa1a1;

/// Value returned by the sibling worker on failure.
const WORKER_FAILURE: usize = usize::MAX;

//==================================================================================================
// Globals
//==================================================================================================

/// Set once the sibling worker has started executing.
static WORKER_STARTED: AtomicBool = AtomicBool::new(false);

/// Set by the main thread to release the sibling worker from its busy loop.
static WORKER_RELEASE: AtomicBool = AtomicBool::new(false);

/// Set by the sibling worker right before it returns (i.e., it exhausted its busy budget).
static WORKER_FINISHED: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Public Functions
//==================================================================================================

/// Runs the per-thread timer-alarm starvation regression test.
pub fn run() -> Result<(), Error> {
    test_timed_sleep_with_busy_sibling()
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Verifies that a thread's timed `sleep()` fires from its own alarm even while a sibling thread in
/// the same process is continuously runnable (CPU-bound).
///
/// Before the fix, the scheduler serviced alarms only for fully-suspended processes. A busy sibling
/// kept the process in the ready queue, so the sleeper's alarm was never serviced until the sibling
/// stopped — turning an independent timed sleep into a process-wide barrier.
///
/// The sleeper sleeps for `SLEEPER_TIMEOUT` while the sibling stays runnable for up to
/// `BUSY_BUDGET` (much longer). If the alarm is serviced correctly, the sleeper wakes while the
/// sibling is still running. If the bug is present, the sleeper cannot wake until the sibling
/// exhausts its budget and exits — by which point the sibling has already finished.
fn test_timed_sleep_with_busy_sibling() -> Result<(), Error> {
    WORKER_STARTED.store(false, Ordering::SeqCst);
    WORKER_RELEASE.store(false, Ordering::SeqCst);
    WORKER_FINISHED.store(false, Ordering::SeqCst);

    let worker: KernelThread = KernelThread::spawn(busy_sibling, 0)?;

    // Ensure the sibling is always released and joined, even if a fallible operation below returns
    // early via `?`. Otherwise the busy sibling could keep running until `BUSY_BUDGET` elapses and
    // interfere with subsequent tests.
    let mut guard: WorkerGuard = WorkerGuard::new(worker);

    // Wait until the sibling is actually running so that it is in the ready set before we sleep.
    while !WORKER_STARTED.load(Ordering::SeqCst) {
        __kcall_sched_yield()?;
    }

    // Sleep for a short, fixed duration while the sibling keeps the process runnable.
    let before: SystemTime = monotonic_now()?;
    __kcall_sleep(SLEEPER_TIMEOUT)?;
    let after: SystemTime = monotonic_now()?;

    // Capture whether the sibling already finished. Its budget is far larger than our sleep, so it
    // must still be running unless our alarm was starved until the sibling exited.
    let sibling_finished_early: bool = WORKER_FINISHED.load(Ordering::SeqCst);

    // Release the sibling and join it so the thread is cleaned up regardless of the outcome.
    let retval: usize = guard.release_and_join()?;

    // Primary check: the timed sleep must not have been starved by the busy sibling.
    if sibling_finished_early {
        return Err(Error::new(
            ErrorCode::OperationTimedOut,
            "timed sleep was starved by a busy sibling thread (per-thread alarm not serviced)",
        ));
    }

    // Secondary check: the sleep must not return before its requested deadline.
    match after.checked_sub(&before) {
        Ok(elapsed) => {
            if elapsed < SLEEPER_TIMEOUT {
                return Err(Error::new(
                    ErrorCode::OperationTimedOut,
                    "timed sleep returned before its deadline",
                ));
            }
        },
        Err(_) => {
            return Err(Error::new(ErrorCode::TryAgain, "monotonic clock regressed during sleep"));
        },
    }

    // Sanity: the sibling worker completed successfully.
    if retval != WORKER_OK {
        return Err(Error::new(ErrorCode::InvalidArgument, "sibling worker reported failure"));
    }

    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

/// RAII guard that releases and joins the sibling worker thread on drop, guaranteeing the worker is
/// cleaned up even when the test returns early via `?`.
struct WorkerGuard {
    worker: Option<KernelThread>,
}

impl WorkerGuard {
    /// Creates a new guard taking ownership of the spawned worker thread.
    fn new(worker: KernelThread) -> Self {
        Self {
            worker: Some(worker),
        }
    }

    /// Releases the worker from its busy loop and joins it, returning its exit value. After this
    /// call the guard's drop is a no-op.
    fn release_and_join(&mut self) -> Result<usize, Error> {
        WORKER_RELEASE.store(true, Ordering::SeqCst);
        match self.worker.take() {
            Some(worker) => worker.join(),
            None => Ok(WORKER_OK),
        }
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        WORKER_RELEASE.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Entry point for the CPU-bound sibling worker.
extern "C" fn busy_sibling(_arg: usize) -> usize {
    busy_sibling_impl().unwrap_or(WORKER_FAILURE)
}

/// Stays continuously runnable (CPU-bound) until released by the main thread or until `BUSY_BUDGET`
/// elapses, whichever comes first. Yielding each iteration keeps the thread in the ready set,
/// reproducing the exact condition that starves a sibling's timed alarm on a buggy kernel.
fn busy_sibling_impl() -> Result<usize, Error> {
    // Signal that the worker is running before any fallible call, so the main thread never spins
    // indefinitely in its `WORKER_STARTED` wait loop if an early operation fails.
    WORKER_STARTED.store(true, Ordering::SeqCst);

    let deadline: SystemTime = deadline_from_now(BUSY_BUDGET)?;

    loop {
        if WORKER_RELEASE.load(Ordering::SeqCst) {
            break;
        }
        if monotonic_now()? >= deadline {
            break;
        }
        __kcall_sched_yield()?;
    }

    WORKER_FINISHED.store(true, Ordering::SeqCst);
    Ok(WORKER_OK)
}
