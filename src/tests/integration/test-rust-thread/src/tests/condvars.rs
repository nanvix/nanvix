// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::{
    deadline_from_now,
    raw_entry_address,
    raw_pointer_address,
};
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::{
    ptr,
    sync::atomic::{
        AtomicU8,
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
            __kcall_join_thread,
            __kcall_lock_mutex,
            __kcall_signal_cond,
            __kcall_unlock_mutex,
            __kcall_wait_cond,
        },
        sched::__kcall_sched_yield,
    },
    pm::{
        ConditionAddress,
        MutexAddress,
        ThreadCreateArgs,
    },
};
use ::sysapi::{
    pthread::{
        PTHREAD_COND_INITIALIZER,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sys_types::{
        pthread_cond_t,
        pthread_mutex_t,
    },
};
use ::syscall::safe::mem::stack::Stack;

//==================================================================================================
// Globals
//==================================================================================================

static mut TEST_COND: pthread_cond_t = PTHREAD_COND_INITIALIZER;
static mut COND_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
static WAIT_STATE: AtomicU8 = AtomicU8::new(0);

fn cond_addr() -> ConditionAddress {
    let ptr: *mut pthread_cond_t = ptr::addr_of_mut!(TEST_COND);
    ConditionAddress::from(raw_pointer_address(ptr))
}

fn mutex_addr() -> MutexAddress {
    let ptr: *mut pthread_mutex_t = ptr::addr_of_mut!(COND_MUTEX);
    MutexAddress::from(raw_pointer_address(ptr))
}

fn reset_sync_primitives() {
    unsafe {
        TEST_COND = PTHREAD_COND_INITIALIZER;
        COND_MUTEX = PTHREAD_MUTEX_INITIALIZER;
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Validates condition variable signaling and timeout behavior.
pub fn run() -> Result<(), Error> {
    test_condition_signal()?;
    test_condition_timeout()?;
    Ok(())
}

fn test_condition_signal() -> Result<(), Error> {
    reset_sync_primitives();
    WAIT_STATE.store(0, Ordering::Relaxed);

    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: raw_entry_address(condition_wait_worker),
        user_fn_arg1: 0,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    };
    let tid = __kcall_create_thread(&mut args)?;

    while WAIT_STATE.load(Ordering::Acquire) == 0 {
        __kcall_sched_yield()?;
    }

    __kcall_lock_mutex(mutex_addr(), None)?;
    let awakened = __kcall_signal_cond(cond_addr(), false)?;
    assert_eq!(awakened, 1, "expected to wake exactly one waiter");
    __kcall_unlock_mutex(mutex_addr())?;

    let mut retval: usize = 0;
    __kcall_join_thread(tid, &mut retval)?;
    drop(stack);
    assert_eq!(retval, 0, "condition worker returned unexpected code");
    assert_eq!(WAIT_STATE.load(Ordering::Acquire), 2, "worker never completed wait");
    Ok(())
}

fn test_condition_timeout() -> Result<(), Error> {
    reset_sync_primitives();
    WAIT_STATE.store(0, Ordering::Relaxed);

    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: raw_entry_address(condition_timeout_worker),
        user_fn_arg1: 0,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    };
    let tid = __kcall_create_thread(&mut args)?;

    let mut retval: usize = 0;
    __kcall_join_thread(tid, &mut retval)?;
    drop(stack);
    assert_eq!(retval, 1, "condition timeout worker returned unexpected code");
    assert_eq!(WAIT_STATE.load(Ordering::Acquire), 3, "timeout path did not run");
    Ok(())
}

extern "C" fn condition_wait_worker(_arg: usize) -> usize {
    condition_wait_worker_impl().unwrap_or_else(|err| panic!("condition_wait_worker: {err:?}"))
}

fn condition_wait_worker_impl() -> Result<usize, Error> {
    __kcall_lock_mutex(mutex_addr(), None)?;
    WAIT_STATE.store(1, Ordering::Release);
    __kcall_wait_cond(cond_addr(), mutex_addr(), None)?;
    WAIT_STATE.store(2, Ordering::Release);
    __kcall_unlock_mutex(mutex_addr())?;
    Ok(0)
}

extern "C" fn condition_timeout_worker(_arg: usize) -> usize {
    condition_timeout_worker_impl()
        .unwrap_or_else(|err| panic!("condition_timeout_worker: {err:?}"))
}

fn condition_timeout_worker_impl() -> Result<usize, Error> {
    __kcall_lock_mutex(mutex_addr(), None)?;
    WAIT_STATE.store(1, Ordering::Release);
    let deadline = deadline_from_now(Duration::from_millis(100))?;
    match __kcall_wait_cond(cond_addr(), mutex_addr(), Some(deadline)) {
        Err(err) => {
            assert_eq!(err.code, ErrorCode::OperationTimedOut, "unexpected error code");
        },
        Ok(_) => panic!("wait_cond() should time out"),
    }
    WAIT_STATE.store(3, Ordering::Release);
    __kcall_unlock_mutex(mutex_addr())?;
    Ok(1)
}
