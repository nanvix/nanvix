// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::KernelThread;
use ::sys::error::Error;
use ::sysapi::sys_types::pthread_t;
use ::syscall::pthread::pthread_self;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Validates `pthread_self()` behavior (ports self.c).
pub fn run() -> Result<(), Error> {
    test_pthread_self_consistency()?;
    test_pthread_self_worker_distinct()?;
    Ok(())
}

/// Verifies that `pthread_self()` returns a consistent, positive identifier for the calling thread.
fn test_pthread_self_consistency() -> Result<(), Error> {
    let self_id: pthread_t = pthread_self();
    assert!(self_id > 0, "thread identifier must be positive");

    let self_id_again: pthread_t = pthread_self();
    assert_eq!(self_id, self_id_again, "pthread_self must return consistent value");

    Ok(())
}

/// Verifies that a worker thread receives a distinct identifier from the main thread.
fn test_pthread_self_worker_distinct() -> Result<(), Error> {
    let main_id: pthread_t = pthread_self();

    let thread = KernelThread::spawn(self_worker_entry, 0)?;
    let worker_id_raw: usize = thread.join()?;

    #[allow(clippy::as_conversions)]
    let main_id_raw: usize = main_id as usize;
    assert_ne!(worker_id_raw, main_id_raw, "worker must have distinct thread identifier");

    Ok(())
}

extern "C" fn self_worker_entry(_arg: usize) -> usize {
    let worker_self: pthread_t = pthread_self();
    #[allow(clippy::as_conversions)]
    let id: usize = worker_self as usize;
    id
}
