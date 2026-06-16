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
};

//==================================================================================================
// Constants
//==================================================================================================

const CHURN_WORKERS: usize = 4;
const CHURN_ITERATIONS: usize = 128;
const CHURN_WORKER_FAILURE: usize = usize::MAX;

//==================================================================================================
// Globals
//==================================================================================================

static CHURN_TOTAL: AtomicUsize = AtomicUsize::new(0);
static CHURN_ERROR_CODE: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Exercises contended mutex paths by spawning several workers that repeatedly lock/unlock the same
/// mutex, approximating heavy contention on shared kernel structures.
///
/// # Returns
///
/// `Ok(())` on success or an error if mutex operations or thread lifecycle calls fail.
///
pub fn run() -> Result<(), StressError> {
    reset_stress_mutex();
    CHURN_TOTAL.store(0, Ordering::Relaxed);

    let mut tids: ::alloc::vec::Vec<ThreadIdentifier> =
        ::alloc::vec::Vec::with_capacity(CHURN_WORKERS);
    let mut stacks: ::alloc::vec::Vec<WorkerStack> =
        ::alloc::vec::Vec::with_capacity(CHURN_WORKERS);

    for worker_id in 0..CHURN_WORKERS {
        let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_THREAD_STACK_SIZE)?;
        let mut args: ThreadCreateArgs = thread_args(&stack, mutex_churn_worker, worker_id);
        let tid: ThreadIdentifier = __kcall_create_thread(&mut args)?;
        tids.push(tid);
        stacks.push(stack);
    }

    for (tid, stack) in tids.into_iter().zip(stacks) {
        let mut retval: usize = 0;
        __kcall_join_thread(tid, &mut retval)?;
        drop(stack);
        if retval == CHURN_WORKER_FAILURE {
            let code_raw: usize = CHURN_ERROR_CODE.swap(0, Ordering::AcqRel);
            let code: ErrorCode = error_code_from_usize(code_raw);
            return Err(Error::new(code, "mutex churn worker failed"));
        }

        if retval != CHURN_ITERATIONS {
            return Err(Error::new(ErrorCode::InvalidArgument, "churn worker iteration mismatch"));
        }
    }

    let observed_total: usize = CHURN_TOTAL.load(Ordering::Acquire);
    if observed_total != CHURN_WORKERS * CHURN_ITERATIONS {
        return Err(Error::new(ErrorCode::InvalidArgument, "mutex churn missed increments"));
    }

    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for mutex churn workers that repeatedly lock and unlock the shared mutex.
///
/// # Parameters
///
/// - `worker_id`: Identifier used to diversify yield cadence.
///
/// # Returns
///
/// Number of iterations the worker completed.
extern "C" fn mutex_churn_worker(worker_id: usize) -> usize {
    match mutex_churn_worker_impl(worker_id) {
        Ok(iterations) => iterations,
        Err(err) => {
            CHURN_ERROR_CODE.store(error_code_to_usize(err.code), Ordering::Release);
            CHURN_WORKER_FAILURE
        },
    }
}

///
/// # Description
///
/// Implements the mutex churn loop by contending on the shared mutex and yielding periodically.
///
/// # Parameters
///
/// - `worker_id`: Identifier used to diversify yield cadence.
///
/// # Returns
///
/// `Ok(count)` on success where `count` is the number of iterations completed.
fn mutex_churn_worker_impl(worker_id: usize) -> Result<usize, Error> {
    for iteration in 0..CHURN_ITERATIONS {
        __kcall_lock_mutex(stress_mutex_addr(), None)?;
        CHURN_TOTAL.fetch_add(1, Ordering::AcqRel);
        __kcall_unlock_mutex(stress_mutex_addr())?;

        if (iteration ^ worker_id) & 0x7 == 0 {
            __kcall_sched_yield()?;
        }
    }

    Ok(CHURN_ITERATIONS)
}
