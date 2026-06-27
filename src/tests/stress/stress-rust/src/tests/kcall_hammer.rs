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
    reset_stress_mutex,
    stress_mutex_addr,
    thread_args,
};
use ::core::sync::atomic::{
    AtomicUsize,
    Ordering,
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
            __kcall_lock_mutex,
            __kcall_unlock_mutex,
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

const KCALL_HAMMER_WORKERS: usize = 6;
const KCALL_HAMMER_ITERATIONS: usize = 96;
const KCALL_HAMMER_FAILURE: usize = usize::MAX;

//==================================================================================================
// Globals
//==================================================================================================

static KCALL_HAMMER_PROGRESS: AtomicUsize = AtomicUsize::new(0);
static KCALL_HAMMER_ERROR_CODE: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Hammers fast kcall paths (`gettime`, mutex ops) from several threads to emulate timer-heavy
/// telemetry collectors hitting the kernel concurrently.
///
/// # Returns
///
/// `Ok(())` on success or an error if thread lifecycle or kcall operations fail.
///
pub fn run() -> Result<(), StressError> {
    reset_stress_mutex();
    KCALL_HAMMER_PROGRESS.store(0, Ordering::Relaxed);

    let mut tids: ::alloc::vec::Vec<ThreadIdentifier> =
        ::alloc::vec::Vec::with_capacity(KCALL_HAMMER_WORKERS);
    let mut stacks: ::alloc::vec::Vec<WorkerStack> =
        ::alloc::vec::Vec::with_capacity(KCALL_HAMMER_WORKERS);

    for worker_id in 0..KCALL_HAMMER_WORKERS {
        let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_THREAD_STACK_SIZE)?;
        let mut args: ThreadCreateArgs = thread_args(&stack, kcall_hammer_worker, worker_id);
        let tid: ThreadIdentifier = __kcall_create_thread(&mut args)?;
        tids.push(tid);
        stacks.push(stack);
    }

    for (tid, stack) in tids.into_iter().zip(stacks) {
        let mut retval: usize = 0;
        __kcall_join_thread(tid, &mut retval)?;
        drop(stack);
        if retval == KCALL_HAMMER_FAILURE {
            let code_raw: usize = KCALL_HAMMER_ERROR_CODE.swap(0, Ordering::AcqRel);
            let code: ErrorCode = error_code_from_usize(code_raw);
            return Err(Error::new(code, "kcall hammer worker failed"));
        }

        if retval != KCALL_HAMMER_ITERATIONS {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "hammer worker iterations mismatch",
            ));
        }
    }

    let observed_progress: usize = KCALL_HAMMER_PROGRESS.load(Ordering::Acquire);
    if observed_progress != KCALL_HAMMER_WORKERS * KCALL_HAMMER_ITERATIONS {
        return Err(Error::new(ErrorCode::InvalidArgument, "kcall hammer missed iterations"));
    }

    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for kcall hammer workers that mix mutex and time syscalls.
///
/// # Parameters
///
/// - `worker_id`: Identifier used to vary yield cadence.
///
/// # Returns
///
/// Number of iterations completed by the worker.
extern "C" fn kcall_hammer_worker(worker_id: usize) -> usize {
    match kcall_hammer_worker_impl(worker_id) {
        Ok(iterations) => iterations,
        Err(err) => {
            KCALL_HAMMER_ERROR_CODE.store(error_code_to_usize(err.code), Ordering::Release);
            KCALL_HAMMER_FAILURE
        },
    }
}

///
/// # Description
///
/// Implements the kcall hammer loop by locking a shared mutex and reading system time.
///
/// # Parameters
///
/// - `worker_id`: Identifier used to vary yield cadence.
///
/// # Returns
///
/// `Ok(count)` on success where `count` is the number of iterations completed.
fn kcall_hammer_worker_impl(worker_id: usize) -> Result<usize, Error> {
    for iteration in 0..KCALL_HAMMER_ITERATIONS {
        __kcall_lock_mutex(stress_mutex_addr(), None)?;
        let mut now: SystemTime = SystemTime::default();
        __kcall_gettime(&mut now)?;
        __kcall_unlock_mutex(stress_mutex_addr())?;

        KCALL_HAMMER_PROGRESS.fetch_add(1, Ordering::AcqRel);

        if (iteration + worker_id) & 0x3 == 0 {
            __kcall_sched_yield()?;
        }
    }

    Ok(KCALL_HAMMER_ITERATIONS)
}
