// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::{
    KernelThread,
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
        MutexAddress,
        ThreadCreateArgs,
    },
    time::SystemTime,
};
use ::sysapi::{
    pthread::PTHREAD_MUTEX_INITIALIZER,
    sys_types::{
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
};
use ::syscall::{
    pthread::{
        pthread_mutex_destroy,
        pthread_mutex_init,
        pthread_mutex_lock,
        pthread_mutex_trylock,
        pthread_mutex_unlock,
    },
    safe::mem::stack::Stack,
};

//==================================================================================================
// Globals
//==================================================================================================

static mut TEST_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
static WORKER_STATE: AtomicU8 = AtomicU8::new(0);

// Globals for dynamic init test.
static mut DYN_MUTEX: pthread_mutex_t = 0;
static DYN_INITIALIZED: AtomicU8 = AtomicU8::new(0);

// Globals for trylock test.
static mut TRYLOCK_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

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
    test_mutex_dynamic_init()?;
    test_mutex_trylock()?;
    Ok(())
}

fn test_mutex_contention() -> Result<(), Error> {
    reset_test_mutex();
    WORKER_STATE.store(0, Ordering::Relaxed);

    let mutex_addr: MutexAddress = test_mutex_addr();
    __kcall_lock_mutex(mutex_addr, None)?;

    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: raw_entry_address(mutex_worker),
        user_fn_arg1: usize::from(mutex_addr),
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    };
    let tid = __kcall_create_thread(&mut args)?;

    while WORKER_STATE.load(Ordering::Acquire) == 0 {
        __kcall_sched_yield()?;
    }
    assert_eq!(WORKER_STATE.load(Ordering::Acquire), 1, "worker should be blocked");

    __kcall_unlock_mutex(mutex_addr)?;

    let mut retval: usize = 0;
    __kcall_join_thread(tid, &mut retval)?;
    drop(stack);
    assert_eq!(retval, 0, "mutex worker returned unexpected status");
    assert_eq!(WORKER_STATE.load(Ordering::Acquire), 2, "worker never executed critical section");
    Ok(())
}

fn test_mutex_timeout() -> Result<(), Error> {
    reset_test_mutex();

    let mutex_addr: MutexAddress = test_mutex_addr();
    __kcall_lock_mutex(mutex_addr, None)?;

    match __kcall_lock_mutex(mutex_addr, Some(SystemTime::EPOCH)) {
        Err(err) => {
            assert_eq!(err.code, ErrorCode::OperationTimedOut, "unexpected error code");
        },
        Ok(_) => panic!("second lock must time out immediately"),
    }

    __kcall_unlock_mutex(mutex_addr)?;
    Ok(())
}

extern "C" fn mutex_worker(raw_addr: usize) -> usize {
    let mutex_addr: MutexAddress = MutexAddress::from(raw_addr);
    mutex_worker_impl(mutex_addr).unwrap_or_else(|err| panic!("mutex_worker: {err:?}"))
}

fn mutex_worker_impl(mutex_addr: MutexAddress) -> Result<usize, Error> {
    WORKER_STATE.store(1, Ordering::Release);
    __kcall_lock_mutex(mutex_addr, None)?;
    WORKER_STATE.store(2, Ordering::Release);
    __kcall_unlock_mutex(mutex_addr)?;
    Ok(0)
}

//==================================================================================================
// Dynamic Init / Destroy (ports mutex_dynamic_init.c)
//==================================================================================================

fn test_mutex_dynamic_init() -> Result<(), Error> {
    // Run two init/destroy cycles to verify reusability.
    for _ in 0..2_u32 {
        unsafe {
            DYN_MUTEX = 0;
        }
        DYN_INITIALIZED.store(0, Ordering::Relaxed);

        let attr: pthread_mutexattr_t = pthread_mutexattr_t::default();
        // SAFETY: single-threaded access during initialization.
        unsafe {
            pthread_mutex_init(&mut *ptr::addr_of_mut!(DYN_MUTEX), &attr)?;
        }

        let thread = KernelThread::spawn(dyn_mutex_worker, 0)?;

        // Poll until the worker signals initialization.
        loop {
            // SAFETY: coordinated access through the mutex.
            unsafe {
                pthread_mutex_lock(&mut *ptr::addr_of_mut!(DYN_MUTEX))?;
            }
            let ready = DYN_INITIALIZED.load(Ordering::Acquire);
            unsafe {
                pthread_mutex_unlock(&mut *ptr::addr_of_mut!(DYN_MUTEX))?;
            }
            if ready != 0 {
                break;
            }
            __kcall_sched_yield()?;
        }

        let retval = thread.join()?;
        assert_eq!(retval, 0xdeadbeef, "dynamic mutex worker returned unexpected status");

        // SAFETY: single-threaded access during destruction.
        unsafe {
            pthread_mutex_destroy(&mut *ptr::addr_of_mut!(DYN_MUTEX))?;
        }
    }

    Ok(())
}

extern "C" fn dyn_mutex_worker(_arg: usize) -> usize {
    dyn_mutex_worker_impl().unwrap_or_else(|err| panic!("dyn_mutex_worker: {err:?}"))
}

fn dyn_mutex_worker_impl() -> Result<usize, Error> {
    // SAFETY: mutex already initialized; access via lock/unlock is coordinated.
    unsafe {
        pthread_mutex_lock(&mut *ptr::addr_of_mut!(DYN_MUTEX))?;
    }
    DYN_INITIALIZED.store(1, Ordering::Release);
    unsafe {
        pthread_mutex_unlock(&mut *ptr::addr_of_mut!(DYN_MUTEX))?;
    }
    Ok(0xdeadbeef)
}

//==================================================================================================
// Trylock (ports mutex_trylock.c)
//==================================================================================================

fn test_mutex_trylock() -> Result<(), Error> {
    unsafe {
        TRYLOCK_MUTEX = PTHREAD_MUTEX_INITIALIZER;
    }

    // Main thread locks the mutex.
    // SAFETY: single-threaded access before spawning worker.
    unsafe {
        pthread_mutex_lock(&mut *ptr::addr_of_mut!(TRYLOCK_MUTEX))?;
    }

    // Spawn a worker that attempts trylock on the already-locked mutex.
    let thread = KernelThread::spawn(trylock_worker, 0)?;

    let retval = thread.join()?;
    assert_eq!(retval, 0, "trylock worker returned unexpected status");

    // Release the mutex.
    unsafe {
        pthread_mutex_unlock(&mut *ptr::addr_of_mut!(TRYLOCK_MUTEX))?;
    }

    Ok(())
}

extern "C" fn trylock_worker(_arg: usize) -> usize {
    trylock_worker_impl().unwrap_or_else(|err| panic!("trylock_worker: {err:?}"))
}

fn trylock_worker_impl() -> Result<usize, Error> {
    // Attempt to lock a mutex already held by the main thread.
    // SAFETY: the mutex is initialized and locked by main; trylock must fail with ResourceBusy.
    let result: Result<(), Error> =
        unsafe { pthread_mutex_trylock(&mut *ptr::addr_of_mut!(TRYLOCK_MUTEX)) };

    match result {
        Err(err) => {
            assert_eq!(err.code, ErrorCode::ResourceBusy, "trylock should return ResourceBusy");
        },
        Ok(_) => panic!("trylock must fail on an already-locked mutex"),
    }

    Ok(0)
}
