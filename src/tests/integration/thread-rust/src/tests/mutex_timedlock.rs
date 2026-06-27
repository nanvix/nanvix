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
    pthread::PTHREAD_MUTEX_INITIALIZER,
    sys_types::pthread_mutex_t,
};
use ::syscall::pthread::{
    pthread_mutex_lock,
    pthread_mutex_timedlock,
    pthread_mutex_unlock,
};

//==================================================================================================
// Globals
//==================================================================================================

static mut TIMED_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all `pthread_mutex_timedlock()` tests.
pub fn run() -> Result<(), Error> {
    test_mutex_timedlock_timeout()?;
    Ok(())
}

/// Verifies that `pthread_mutex_timedlock()` returns `OperationTimedOut` when a worker thread
/// attempts to acquire a mutex that is already held by the main thread past the deadline.
fn test_mutex_timedlock_timeout() -> Result<(), Error> {
    // Reset the mutex.
    unsafe {
        TIMED_MUTEX = PTHREAD_MUTEX_INITIALIZER;
    }

    // Main thread locks the mutex before spawning the worker.
    // SAFETY: single-threaded access before spawning.
    unsafe {
        pthread_mutex_lock(&mut *ptr::addr_of_mut!(TIMED_MUTEX))?;
    }

    // Spawn a worker that tries to acquire the locked mutex with a 1-second deadline.
    let thread = KernelThread::spawn(timedlock_worker, 0xbadcafe)?;

    // Wait for the worker to finish — it should time out and return the expected status.
    let retval = thread.join()?;
    assert_eq!(retval, 0xdeadbeef, "timedlock worker returned unexpected status");

    // Release the mutex.
    unsafe {
        pthread_mutex_unlock(&mut *ptr::addr_of_mut!(TIMED_MUTEX))?;
    }

    Ok(())
}

extern "C" fn timedlock_worker(arg: usize) -> usize {
    timedlock_worker_impl(arg).unwrap_or_else(|err| panic!("timedlock_worker: {err:?}"))
}

fn timedlock_worker_impl(arg: usize) -> Result<usize, Error> {
    assert_eq!(arg, 0xbadcafe, "unexpected worker argument");

    let deadline = deadline_from_now(Duration::from_secs(1))?;

    // Attempt to lock a mutex already held by the main thread — must time out.
    // SAFETY: the mutex is initialized and locked by main; timedlock must fail.
    let result: Result<(), Error> =
        unsafe { pthread_mutex_timedlock(&mut *ptr::addr_of_mut!(TIMED_MUTEX), Some(deadline)) };

    match result {
        Err(err) => {
            assert_eq!(
                err.code,
                ErrorCode::OperationTimedOut,
                "timedlock should return OperationTimedOut"
            );
        },
        Ok(_) => panic!("timedlock must fail on an already-locked mutex"),
    }

    Ok(0xdeadbeef)
}
