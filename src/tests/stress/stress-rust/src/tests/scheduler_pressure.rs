// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::{
    StressError,
    WorkerStack,
    error_code_from_usize,
    error_code_to_usize,
    thread_args,
};
use ::core::{
    sync::atomic::{
        AtomicUsize,
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
        pm::{
            __kcall_create_thread,
            __kcall_gettime,
            __kcall_join_thread,
            __kcall_sleep,
        },
        sched::__kcall_sched_yield,
    },
    pm::{
        ThreadCreateArgs,
        ThreadIdentifier,
    },
    time::SystemTime,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of concurrent worker threads. Chosen to keep many threads simultaneously in the ready
/// queue, forcing the scheduler to select among them repeatedly.
const PRESSURE_WORKERS: usize = 8;

/// Rounds per worker. Each round performs a burst of fast kcalls followed by a brief sleep, causing
/// the thread to cycle through ready → running → sleeping → ready states.
const PRESSURE_ROUNDS: usize = 32;

/// Number of `gettime` + `sched_yield` kcalls per round. High enough to saturate kernel
/// scheduling paths with rapid, back-to-back kernel calls.
const SYSCALLS_PER_ROUND: usize = 64;

/// Sentinel value returned by a worker on failure.
const PRESSURE_FAILURE: usize = usize::MAX;

/// Brief inter-round sleep duration. Short enough to avoid slowing the test, long enough to force
/// a sleep/wakeup transition that exercises the scheduler.
const INTER_ROUND_SLEEP: Duration = Duration::from_micros(64);

//==================================================================================================
// Globals
//==================================================================================================

/// Tracks the total number of completed rounds across all workers.
static PRESSURE_PROGRESS: AtomicUsize = AtomicUsize::new(0);

/// Stores the error code from the first worker that fails.
static PRESSURE_ERROR_CODE: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Stresses the kernel scheduler by running many concurrent threads that cycle rapidly through
/// ready, running, and sleeping states under sustained kernel-call pressure.
///
/// Each worker thread performs tight loops of `gettime()` + `sched_yield()` kcalls interspersed
/// with brief sleeps, forcing frequent ready ↔ sleeping state transitions.
///
/// # Returns
///
/// `Ok(())` on success or an error if any worker fails or the total progress count is wrong.
///
pub fn run() -> Result<(), StressError> {
    PRESSURE_PROGRESS.store(0, Ordering::Relaxed);
    PRESSURE_ERROR_CODE.store(0, Ordering::Relaxed);

    let mut tids: ::alloc::vec::Vec<ThreadIdentifier> =
        ::alloc::vec::Vec::with_capacity(PRESSURE_WORKERS);

    // Allocate all stacks before spawning any threads so that a late allocation failure cannot
    // drop stacks that are already in use by running threads (use-after-free).
    let mut stacks: ::alloc::vec::Vec<WorkerStack> =
        ::alloc::vec::Vec::with_capacity(PRESSURE_WORKERS);
    for _ in 0..PRESSURE_WORKERS {
        stacks.push(WorkerStack::new(::config::memory_layout::USER_THREAD_STACK_SIZE)?);
    }

    // Spawn all workers upfront to maximize concurrent scheduling pressure.
    for (worker_id, stack) in stacks.iter().enumerate() {
        let mut args: ThreadCreateArgs = thread_args(stack, scheduler_pressure_worker, worker_id);
        match __kcall_create_thread(&mut args) {
            Ok(tid) => tids.push(tid),
            Err(e) => {
                // Join already-spawned threads before returning so their stacks remain valid.
                for tid in &tids {
                    let mut retval: usize = 0;
                    let _ = __kcall_join_thread(*tid, &mut retval);
                }
                return Err(e);
            },
        }
    }

    // Join each worker and check for failures. Stacks are kept alive until after the loop.
    for tid in &tids {
        let mut retval: usize = 0;
        __kcall_join_thread(*tid, &mut retval)?;
        if retval == PRESSURE_FAILURE {
            let code_raw: usize = PRESSURE_ERROR_CODE.swap(0, Ordering::Relaxed);
            let code: ErrorCode = error_code_from_usize(code_raw);
            return Err(Error::new(code, "scheduler pressure worker failed"));
        }

        if retval != PRESSURE_ROUNDS {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "scheduler pressure rounds mismatch",
            ));
        }
    }

    // NOTE: `join_thread` provides happens-before synchronization, so `Relaxed` is sufficient.
    let observed_progress: usize = PRESSURE_PROGRESS.load(Ordering::Relaxed);
    if observed_progress != PRESSURE_WORKERS * PRESSURE_ROUNDS {
        return Err(Error::new(ErrorCode::InvalidArgument, "scheduler pressure missed rounds"));
    }

    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for scheduler pressure workers.
///
/// # Parameters
///
/// - `worker_id`: Identifier used to stagger yield cadence across workers.
///
/// # Returns
///
/// Number of completed rounds, or `PRESSURE_FAILURE` on error.
extern "C" fn scheduler_pressure_worker(worker_id: usize) -> usize {
    match scheduler_pressure_worker_impl(worker_id) {
        Ok(rounds) => rounds,
        Err(err) => {
            // Use compare_exchange so that the first error wins if multiple workers fail.
            // NOTE: `join_thread` provides happens-before synchronization, so `Relaxed`
            // suffices for both success and failure orderings.
            let _ = PRESSURE_ERROR_CODE.compare_exchange(
                0,
                error_code_to_usize(err.code),
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            PRESSURE_FAILURE
        },
    }
}

///
/// # Description
///
/// Implements the scheduler pressure loop. Each round performs a burst of fast kcalls then briefly
/// sleeps, cycling the thread through ready → running → sleeping → ready states.
///
/// # Parameters
///
/// - `worker_id`: Identifier used to stagger yield cadence across workers.
///
/// # Returns
///
/// `Ok(rounds)` on success where `rounds` is `PRESSURE_ROUNDS`.
fn scheduler_pressure_worker_impl(worker_id: usize) -> Result<usize, Error> {
    for round in 0..PRESSURE_ROUNDS {
        // Burst of fast kcalls: alternate gettime and yield to saturate the scheduler.
        for iteration in 0..SYSCALLS_PER_ROUND {
            let mut now: SystemTime = SystemTime::default();
            __kcall_gettime(&mut now)?;

            // Stagger yields across workers so that ready-queue membership varies each cycle.
            if (iteration + worker_id + round) & 0x3 == 0 {
                __kcall_sched_yield()?;
            }
        }

        PRESSURE_PROGRESS.fetch_add(1, Ordering::Relaxed);

        // Brief sleep to force a sleeping → wakeup → ready transition, exercising the scheduler's
        // thread-selection logic again from a different starting state.
        __kcall_sleep(INTER_ROUND_SLEEP)?;
    }

    Ok(PRESSURE_ROUNDS)
}
