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
use ::alloc::vec::Vec;
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
    kcall::pm::{
        __kcall_create_thread,
        __kcall_join_thread,
        __kcall_sleep,
    },
    pm::{
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of worker threads spawned concurrently per round.
const WORKERS: usize = 4;

/// Number of rounds to repeat the test for increased race-window coverage.
const ROUNDS: usize = 8;

/// Duration each worker sleeps before exiting, in microseconds. All workers sleep the same
/// amount so their alarms fire close together, maximizing concurrent wakeup contention.
const SLEEP_MICROS: u64 = 1000;

/// Tag returned by every worker on success. Used to detect silent corruption.
const RETURN_TAG: usize = 0xdeadbeef;

/// Sentinel returned when a worker encounters an internal error.
const WORKER_FAILURE: usize = usize::MAX;

//==================================================================================================
// Globals
//==================================================================================================

/// Stores the error code from the last worker failure so the parent can report it.
static ZOMBIE_JOIN_ERROR_CODE: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Stress test for zombie thread preservation across concurrent thread exits.
///
/// Spawns multiple workers that all sleep simultaneously and then exit close together.
/// The parent thread joins every worker and validates its return value. This exercises
/// process state transitions under contention, ensuring that exited threads remain
/// joinable regardless of the order in which siblings wake up and terminate.
///
/// # Returns
///
/// `Ok(())` on success or an error if any `join_thread()` call fails.
///
pub fn run() -> Result<(), StressError> {
    for round in 0..ROUNDS {
        run_round(round)?;
    }
    Ok(())
}

//==================================================================================================
// Private Functions
//==================================================================================================

///
/// # Description
///
/// Executes a single round: spawns [`WORKERS`] threads that all sleep briefly and exit,
/// then joins every worker and validates its return value.
///
/// # Parameters
///
/// - `_round`: Round index (reserved for future per-round variation).
///
fn run_round(_round: usize) -> Result<(), StressError> {
    let mut tids: Vec<ThreadIdentifier> = Vec::with_capacity(WORKERS);
    let mut stacks: Vec<WorkerStack> = Vec::with_capacity(WORKERS);

    // Spawn all workers before joining any, so they sleep concurrently.
    for _worker_id in 0..WORKERS {
        let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_THREAD_STACK_SIZE)?;
        let mut args: ThreadCreateArgs = thread_args(&stack, zombie_join_worker, 0);
        match __kcall_create_thread(&mut args) {
            Ok(tid) => {
                tids.push(tid);
                stacks.push(stack);
            },
            Err(err) => {
                // Join already-spawned threads before propagating the error.
                for (tid, stack) in tids.into_iter().zip(stacks) {
                    let mut retval: usize = 0;
                    let _ = __kcall_join_thread(tid, &mut retval);
                    drop(stack);
                }
                return Err(err);
            },
        }
    }

    // Join every worker and validate return values.
    for (tid, stack) in tids.into_iter().zip(stacks) {
        let mut retval: usize = 0;
        __kcall_join_thread(tid, &mut retval)?;
        drop(stack);

        if retval == WORKER_FAILURE {
            let code_raw: usize = ZOMBIE_JOIN_ERROR_CODE.swap(0, Ordering::AcqRel);
            let code: ErrorCode = error_code_from_usize(code_raw);
            return Err(Error::new(code, "zombie-join worker failed"));
        }

        if retval != RETURN_TAG {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "zombie-join worker returned unexpected value",
            ));
        }
    }

    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for worker threads. Sleeps briefly so all siblings are sleeping
/// simultaneously, then exits with [`RETURN_TAG`].
///
/// # Parameters
///
/// - `_arg`: Unused argument (required by thread entry-point signature).
///
/// # Returns
///
/// [`RETURN_TAG`] on success, or [`WORKER_FAILURE`] on error.
///
extern "C" fn zombie_join_worker(_arg: usize) -> usize {
    match zombie_join_worker_impl() {
        Ok(tag) => tag,
        Err(err) => {
            // Use compare_exchange to preserve the first error code when multiple workers fail.
            let _ = ZOMBIE_JOIN_ERROR_CODE.compare_exchange(
                0,
                error_code_to_usize(err.code),
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
            WORKER_FAILURE
        },
    }
}

///
/// # Description
///
/// Worker body: sleeps for [`SLEEP_MICROS`] microseconds and returns [`RETURN_TAG`].
///
fn zombie_join_worker_impl() -> Result<usize, Error> {
    __kcall_sleep(Duration::from_micros(SLEEP_MICROS))?;
    Ok(RETURN_TAG)
}
