// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::raw_entry_address;
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::sync::atomic::{
    AtomicU8,
    Ordering,
};
use ::sys::{
    error::Error,
    kcall::{
        pm::{
            __kcall_create_thread,
            __kcall_join_thread,
        },
        sched::__kcall_sched_yield,
    },
    pm::ThreadCreateArgs,
};
use ::syscall::safe::mem::stack::Stack;

//==================================================================================================
// Globals
//==================================================================================================

static SCHED_STATE: AtomicU8 = AtomicU8::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Confirms that `sched_yield()` allows other threads to make progress.
pub fn run() -> Result<(), Error> {
    test_sched_yield_progress()?;
    Ok(())
}

fn test_sched_yield_progress() -> Result<(), Error> {
    SCHED_STATE.store(0, Ordering::Relaxed);
    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: raw_entry_address(yield_worker),
        user_fn_arg1: 0,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    };
    let tid = __kcall_create_thread(&mut args)?;

    // Signal the worker to proceed and yield until it observes the change.
    SCHED_STATE.store(1, Ordering::Release);
    for _ in 0..1024 {
        if SCHED_STATE.load(Ordering::Acquire) == 2 {
            break;
        }
        __kcall_sched_yield()?;
    }

    assert_eq!(SCHED_STATE.load(Ordering::Acquire), 2, "worker never observed yield");
    let mut retval: usize = 0;
    __kcall_join_thread(tid, &mut retval)?;
    drop(stack);
    assert_eq!(retval, 0, "yield worker returned unexpected status");
    Ok(())
}

extern "C" fn yield_worker(_arg: usize) -> usize {
    yield_worker_impl().unwrap_or_else(|err| panic!("yield_worker: {err:?}"))
}

fn yield_worker_impl() -> Result<usize, Error> {
    // Wait until the main thread tells us to proceed.
    while SCHED_STATE.load(Ordering::Acquire) != 1 {
        __kcall_sched_yield()?;
    }

    SCHED_STATE.store(2, Ordering::Release);
    Ok(0)
}
