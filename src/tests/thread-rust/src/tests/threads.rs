// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::raw_entry_address;
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::sync::atomic::{
    AtomicBool,
    AtomicUsize,
    Ordering,
};
use ::sys::{
    error::Error,
    kcall::pm::{
        __kcall_create_thread,
        __kcall_getpid,
        __kcall_gettid,
        __kcall_join_thread,
    },
    pm::ThreadCreateArgs,
};
use ::syscall::safe::mem::stack::Stack;

//==================================================================================================
// Constants
//==================================================================================================

const EXPECTED_WORKER_ARG: usize = 0xbadcafe;
const EXPECTED_EXIT_STATUS: usize = 0xdeadbeef;

//==================================================================================================
// Global State
//==================================================================================================

static WORKER_STARTED: AtomicBool = AtomicBool::new(false);
static WORKER_TID: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Executes thread lifecycle tests.
pub fn run() -> Result<(), Error> {
    test_identifiers()?;
    test_create_and_join()?;
    Ok(())
}

fn test_identifiers() -> Result<(), Error> {
    let pid = __kcall_getpid()?;
    assert!(i32::from(pid) > 0, "process identifier must be positive");

    let tid = __kcall_gettid()?;
    assert!(i32::from(tid) > 0, "thread identifier must be positive");
    Ok(())
}

fn test_create_and_join() -> Result<(), Error> {
    WORKER_STARTED.store(false, Ordering::Relaxed);
    WORKER_TID.store(0, Ordering::Relaxed);

    let main_tid_raw: usize = usize::try_from(__kcall_gettid()?)?;

    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: raw_entry_address(worker_thread),
        user_fn_arg1: EXPECTED_WORKER_ARG,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    };
    let tid = __kcall_create_thread(&mut args)?;

    let mut retval: usize = 0;
    __kcall_join_thread(tid, &mut retval)?;
    drop(stack);
    assert_eq!(retval, EXPECTED_EXIT_STATUS, "unexpected worker exit status");
    assert!(WORKER_STARTED.load(Ordering::Acquire), "worker never started execution");

    let worker_tid: usize = WORKER_TID.load(Ordering::Acquire);
    assert_ne!(worker_tid, main_tid_raw, "worker should have a distinct identifier");
    Ok(())
}

extern "C" fn worker_thread(arg: usize) -> usize {
    worker_thread_impl(arg).unwrap_or_else(|err| panic!("worker_thread: {err:?}"))
}

fn worker_thread_impl(arg: usize) -> Result<usize, Error> {
    assert_eq!(arg, EXPECTED_WORKER_ARG, "worker received unexpected argument");

    let tid_raw: usize = usize::try_from(__kcall_gettid()?)?;
    WORKER_TID.store(tid_raw, Ordering::Release);
    WORKER_STARTED.store(true, Ordering::Release);

    Ok(EXPECTED_EXIT_STATUS)
}
