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

const FAN_OUT_ROUNDS: usize = 16;
const FAN_OUT_SPINS: usize = 64;
const FAN_OUT_RETURN_TAG: usize = 0xfeedface;
const FAN_OUT_FAILURE: usize = usize::MAX;

//==================================================================================================
// Globals
//==================================================================================================

static FAN_OUT_COMPLETED: AtomicUsize = AtomicUsize::new(0);
static FAN_OUT_ERROR_CODE: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Stress tests thread fan-out by rapidly creating and joining short-lived workers, mimicking
/// bursty RPC dispatch patterns that fan out work across many helper threads.
///
/// # Returns
///
/// `Ok(())` on success or an error if thread lifecycle or scheduling calls fail.
///
pub fn run() -> Result<(), StressError> {
    FAN_OUT_COMPLETED.store(0, Ordering::Relaxed);

    for round in 0..FAN_OUT_ROUNDS {
        let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_THREAD_STACK_SIZE)?;
        let mut args: ThreadCreateArgs = thread_args(&stack, fan_out_worker, round);
        let tid: ThreadIdentifier = __kcall_create_thread(&mut args)?;

        let mut retval: usize = 0;
        ::sys::kcall::pm::__kcall_join_thread(tid, &mut retval)?;
        drop(stack);
        if retval == FAN_OUT_FAILURE {
            let code_raw: usize = FAN_OUT_ERROR_CODE.swap(0, Ordering::AcqRel);
            let code: ErrorCode = error_code_from_usize(code_raw);
            return Err(Error::new(code, "fan-out worker failed"));
        }

        if retval != (round ^ FAN_OUT_RETURN_TAG) {
            return Err(Error::new(ErrorCode::InvalidArgument, "unexpected fan-out worker code"));
        }
    }

    let observed_completed: usize = FAN_OUT_COMPLETED.load(Ordering::Acquire);
    if observed_completed != FAN_OUT_ROUNDS {
        return Err(Error::new(ErrorCode::InvalidArgument, "not all fan-out workers completed"));
    }

    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for fan-out worker threads that perform a short yield loop and return a tag.
///
/// # Parameters
///
/// - `round`: Identifier for the parent iteration to encode into the return tag.
///
/// # Returns
///
/// Worker completion tag derived from the round.
extern "C" fn fan_out_worker(round: usize) -> usize {
    match fan_out_worker_impl(round) {
        Ok(tag) => tag,
        Err(err) => {
            FAN_OUT_ERROR_CODE.store(error_code_to_usize(err.code), Ordering::Release);
            FAN_OUT_FAILURE
        },
    }
}

///
/// # Description
///
/// Implements the fan-out worker logic by spinning with yields and reporting completion.
///
/// # Parameters
///
/// - `round`: Identifier for the parent iteration to encode into the return tag.
///
/// # Returns
///
/// `Ok(tag)` on success where `tag` encodes the round value.
fn fan_out_worker_impl(round: usize) -> Result<usize, Error> {
    for _ in 0..FAN_OUT_SPINS {
        __kcall_sched_yield()?;
    }
    FAN_OUT_COMPLETED.fetch_add(1, Ordering::AcqRel);
    Ok(round ^ FAN_OUT_RETURN_TAG)
}
