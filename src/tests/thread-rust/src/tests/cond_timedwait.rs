// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::{
    KernelThread,
    deadline_from_now,
};
use ::core::{
    ptr,
    time::Duration,
};
use ::sys::error::{
    Error,
    ErrorCode,
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
use ::syscall::pthread::{
    pthread_cond_timedwait,
    pthread_mutex_lock,
    pthread_mutex_unlock,
};

//==================================================================================================
// Globals
//==================================================================================================

static mut TIMED_COND: pthread_cond_t = PTHREAD_COND_INITIALIZER;
static mut TIMED_COND_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all `pthread_cond_timedwait()` tests.
pub fn run() -> Result<(), Error> {
    test_cond_timedwait_timeout()?;
    Ok(())
}

/// Verifies that `pthread_cond_timedwait()` returns `OperationTimedOut` when no signal arrives
/// before the deadline. A worker thread locks the mutex, waits on the condition variable with a
/// 1-second timeout, and the main thread never signals — so the worker must time out.
fn test_cond_timedwait_timeout() -> Result<(), Error> {
    // Reset synchronization primitives.
    unsafe {
        TIMED_COND = PTHREAD_COND_INITIALIZER;
        TIMED_COND_MUTEX = PTHREAD_MUTEX_INITIALIZER;
    }

    // Spawn a worker that waits on the condition variable with a timeout.
    let thread = KernelThread::spawn(cond_timedwait_worker, 0xbadcafe)?;

    // Never signal the condition variable — let the worker time out.

    // Wait for the worker to finish and verify its exit status.
    let retval = thread.join()?;
    assert_eq!(retval, 0xdeadbeef, "cond_timedwait worker returned unexpected status");

    Ok(())
}

extern "C" fn cond_timedwait_worker(arg: usize) -> usize {
    cond_timedwait_worker_impl(arg).unwrap_or_else(|err| panic!("cond_timedwait_worker: {err:?}"))
}

fn cond_timedwait_worker_impl(arg: usize) -> Result<usize, Error> {
    assert_eq!(arg, 0xbadcafe, "unexpected worker argument");

    let deadline = deadline_from_now(Duration::from_secs(1))?;

    // Lock the mutex, then wait on the condition variable with a timeout.
    // SAFETY: primitives are initialized and no concurrent access before the lock.
    unsafe {
        pthread_mutex_lock(&mut *ptr::addr_of_mut!(TIMED_COND_MUTEX))?;
    }

    // Wait — the main thread never signals, so this must time out.
    let result: Result<(), Error> = unsafe {
        pthread_cond_timedwait(
            &*ptr::addr_of!(TIMED_COND),
            &*ptr::addr_of!(TIMED_COND_MUTEX),
            Some(deadline),
        )
    };

    match result {
        Err(err) => {
            assert_eq!(
                err.code,
                ErrorCode::OperationTimedOut,
                "cond_timedwait should return OperationTimedOut"
            );
        },
        Ok(_) => panic!("cond_timedwait must time out when no signal arrives"),
    }

    unsafe {
        pthread_mutex_unlock(&mut *ptr::addr_of_mut!(TIMED_COND_MUTEX))?;
    }

    Ok(0xdeadbeef)
}
