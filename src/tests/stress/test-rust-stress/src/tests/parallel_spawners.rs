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
        pm::__kcall_create_thread,
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

const CONCURRENT_SPAWNERS: usize = 4;
const SPAWN_CHILDREN_PER_WORKER: usize = 4;
const SPAWN_CHILD_RETURN_TAG: usize = 0xa55a5aa5;
const FAN_OUT_SPINS: usize = 64;
const SPAWN_FAILURE: usize = usize::MAX;

//==================================================================================================
// Globals
//==================================================================================================

static PARALLEL_SPAWNED: AtomicUsize = AtomicUsize::new(0);
static SPAWN_ERROR_CODE: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Stresses thread creation from multiple parents in parallel, similar to a scheduler burst of
/// control-plane agents each forking helper workers simultaneously.
///
/// # Returns
///
/// `Ok(())` on success or an error if thread lifecycle or scheduling calls fail.
///
pub fn run() -> Result<(), StressError> {
    PARALLEL_SPAWNED.store(0, Ordering::Relaxed);

    let mut tids: Vec<ThreadIdentifier> = Vec::with_capacity(CONCURRENT_SPAWNERS);
    let mut stacks: Vec<WorkerStack> = Vec::with_capacity(CONCURRENT_SPAWNERS);

    for worker_id in 0..CONCURRENT_SPAWNERS {
        let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_THREAD_STACK_SIZE)?;
        let mut args: ThreadCreateArgs = thread_args(&stack, spawner_worker, worker_id);
        let tid: ThreadIdentifier = __kcall_create_thread(&mut args)?;
        tids.push(tid);
        stacks.push(stack);
    }

    for (tid, stack) in tids.into_iter().zip(stacks) {
        let mut retval: usize = 0;
        ::sys::kcall::pm::__kcall_join_thread(tid, &mut retval)?;
        drop(stack);
        if retval == SPAWN_FAILURE {
            let code_raw: usize = SPAWN_ERROR_CODE.swap(0, Ordering::AcqRel);
            let code: ErrorCode = error_code_from_usize(code_raw);
            return Err(Error::new(code, "spawner worker failed"));
        }

        if retval != SPAWN_CHILDREN_PER_WORKER {
            return Err(Error::new(ErrorCode::InvalidArgument, "spawner reported wrong count"));
        }
    }

    let observed_spawned: usize = PARALLEL_SPAWNED.load(Ordering::Acquire);
    if observed_spawned != CONCURRENT_SPAWNERS * SPAWN_CHILDREN_PER_WORKER {
        return Err(Error::new(ErrorCode::InvalidArgument, "not all spawned children completed"));
    }

    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for parent spawner threads that create child workers in parallel.
///
/// # Parameters
///
/// - `worker_id`: Identifier used to vary child signatures and yield cadence.
///
/// # Returns
///
/// Number of child workers launched by this spawner.
extern "C" fn spawner_worker(worker_id: usize) -> usize {
    match spawner_worker_impl(worker_id) {
        Ok(count) => count,
        Err(err) => {
            SPAWN_ERROR_CODE.store(error_code_to_usize(err.code), Ordering::Release);
            SPAWN_FAILURE
        },
    }
}

///
/// # Description
///
/// Implements the spawner logic by launching children and aggregating their completion tags.
///
/// # Parameters
///
/// - `worker_id`: Identifier used to vary child signatures and yield cadence.
///
/// # Returns
///
/// `Ok(count)` on success where `count` is the number of spawned children.
fn spawner_worker_impl(worker_id: usize) -> Result<usize, Error> {
    for child_index in 0..SPAWN_CHILDREN_PER_WORKER {
        let signature: usize = (worker_id << 8) | child_index;
        let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_THREAD_STACK_SIZE)?;
        let mut args: ThreadCreateArgs = thread_args(&stack, spawn_child_worker, signature);
        let tid: ThreadIdentifier = __kcall_create_thread(&mut args)?;

        let mut retval: usize = 0;
        ::sys::kcall::pm::__kcall_join_thread(tid, &mut retval)?;
        drop(stack);
        assert_eq!(retval, signature ^ SPAWN_CHILD_RETURN_TAG, "child returned wrong signature");
        PARALLEL_SPAWNED.fetch_add(1, Ordering::AcqRel);
    }

    Ok(SPAWN_CHILDREN_PER_WORKER)
}

///
/// # Description
///
/// Entry point for child workers spawned by the parallel spawners.
///
/// # Parameters
///
/// - `signature`: Encoded identifier derived from parent and child indices.
///
/// # Returns
///
/// Tagged completion value for the child worker.
extern "C" fn spawn_child_worker(signature: usize) -> usize {
    match spawn_child_worker_impl(signature) {
        Ok(tag) => tag,
        Err(err) => {
            SPAWN_ERROR_CODE.store(error_code_to_usize(err.code), Ordering::Release);
            SPAWN_FAILURE
        },
    }
}

///
/// # Description
///
/// Implements the child worker by yielding briefly before returning its signature tag.
///
/// # Parameters
///
/// - `signature`: Encoded identifier derived from parent and child indices.
///
/// # Returns
///
/// `Ok(tag)` on success where `tag` combines the signature and return marker.
fn spawn_child_worker_impl(signature: usize) -> Result<usize, Error> {
    for _ in 0..FAN_OUT_SPINS {
        __kcall_sched_yield()?;
    }
    Ok(signature ^ SPAWN_CHILD_RETURN_TAG)
}
