// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::{
    raw_entry_address,
    raw_pointer_address,
};
use ::config::memory_layout::USER_STACK_SIZE;
use ::core::{
    ptr,
    sync::atomic::{
        AtomicU8,
        Ordering,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        pm::{
            create_thread,
            join_thread,
            lock_mutex,
            unlock_mutex,
        },
        sched::sched_yield,
    },
    pm::{
        MutexAddress,
        ThreadCreateArgs,
    },
    time::SystemTime,
};
use ::sysapi::{
    pthread::PTHREAD_MUTEX_INITIALIZER,
    sys_types::pthread_mutex_t,
};
use ::syscall::safe::mem::stack::Stack;

//==================================================================================================
// Globals
//==================================================================================================

static mut TEST_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
static WORKER_STATE: AtomicU8 = AtomicU8::new(0);

fn test_mutex_addr() -> MutexAddress {
    let ptr: *mut pthread_mutex_t = ptr::addr_of_mut!(TEST_MUTEX);
    MutexAddress::from(raw_pointer_address(ptr))
}

fn reset_test_mutex() {
    unsafe {
        TEST_MUTEX = PTHREAD_MUTEX_INITIALIZER;
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Exercises mutex locking paths directly through `lock_mutex()`/`unlock_mutex()`.
pub fn run() -> Result<(), Error> {
    test_mutex_contention()?;
    test_mutex_timeout()?;
    Ok(())
}

fn test_mutex_contention() -> Result<(), Error> {
    reset_test_mutex();
    WORKER_STATE.store(0, Ordering::Relaxed);

    let mutex_addr: MutexAddress = test_mutex_addr();
    lock_mutex(mutex_addr, None)?;

    let stack: Stack = Stack::new(USER_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: raw_entry_address(mutex_worker),
        user_fn_arg1: usize::from(mutex_addr),
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    };
    let tid = create_thread(&mut args)?;

    while WORKER_STATE.load(Ordering::Acquire) == 0 {
        sched_yield()?;
    }
    assert_eq!(WORKER_STATE.load(Ordering::Acquire), 1, "worker should be blocked");

    unlock_mutex(mutex_addr)?;

    let mut retval: usize = 0;
    join_thread(tid, &mut retval)?;
    drop(stack);
    assert_eq!(retval, 0, "mutex worker returned unexpected status");
    assert_eq!(WORKER_STATE.load(Ordering::Acquire), 2, "worker never executed critical section");
    Ok(())
}

fn test_mutex_timeout() -> Result<(), Error> {
    reset_test_mutex();

    let mutex_addr: MutexAddress = test_mutex_addr();
    lock_mutex(mutex_addr, None)?;

    match lock_mutex(mutex_addr, Some(SystemTime::EPOCH)) {
        Err(err) => {
            assert_eq!(err.code, ErrorCode::OperationTimedOut, "unexpected error code");
        },
        Ok(_) => panic!("second lock must time out immediately"),
    }

    unlock_mutex(mutex_addr)?;
    Ok(())
}

extern "C" fn mutex_worker(raw_addr: usize) -> usize {
    let mutex_addr: MutexAddress = MutexAddress::from(raw_addr);
    mutex_worker_impl(mutex_addr).unwrap_or_else(|err| panic!("mutex_worker: {err:?}"))
}

fn mutex_worker_impl(mutex_addr: MutexAddress) -> Result<usize, Error> {
    WORKER_STATE.store(1, Ordering::Release);
    lock_mutex(mutex_addr, None)?;
    WORKER_STATE.store(2, Ordering::Release);
    unlock_mutex(mutex_addr)?;
    Ok(0)
}
