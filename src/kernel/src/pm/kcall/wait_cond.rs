// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::{
        condvar::Condvar,
        mutex::{
            Mutex,
            MutexGuard,
        },
    },
    ProcessManager,
    SleepError,
};
use ::sys::pm::{
    ConditionAddress,
    MutexAddress,
    ProcessIdentifier,
    ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Waits on a condition variable.
///
/// # Parameters
///
/// - `pid`: Process identifier.
/// - `tid`: Thread identifier.
/// - `cond_addr`: Address of the condition variable.
/// - `mutex_addr`: Address of the mutex.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Upon failure, a negative error code is returned
/// instead.
///
/// # Safety
///
/// This function is unsafe because:
/// - It operates on global variables.
/// - It may panic.
/// - It may block the calling thread until it is woken up by another thread.
///
/// This function is safe to use if and only if the following conditions are met:
/// - The calling process is not the kernel process.
/// - This function is invoked without holding any resources.
/// - The calling process does not hold a reference to the process manager.
///
pub unsafe fn wait_cond(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    cond_addr: usize,
    mutex_addr: usize,
) -> Result<(), SleepError> {
    trace!(
        "wait_cond(): pid={:?}, tid={:?}, cond_addr={:#x?}, mutex_addr={:#x?}",
        pid,
        tid,
        cond_addr,
        mutex_addr
    );
    // Unpack kernel call arguments.
    let cond_addr: ConditionAddress = ConditionAddress::from(cond_addr);
    let mutex_addr: MutexAddress = MutexAddress::from(mutex_addr);

    {
        ProcessManager::take_mutex_guard(pid, tid, mutex_addr).map_err(SleepError::Generic)?;
        // The mutex guard is dropped, causing threads to be notified.
    }

    {
        let cond: Condvar = ProcessManager::get_cond(cond_addr).map_err(SleepError::Generic)?;
        cond.wait()?;
        // The condition variable is dropped, causing its reference count to decrease.
    }
    ProcessManager::put_cond(cond_addr).map_err(SleepError::Generic)?;

    // Reacquire the mutex.
    let mutex: Mutex = ProcessManager::get_mutex(mutex_addr).map_err(SleepError::Generic)?;
    let guard: MutexGuard = mutex.lock()?;
    ProcessManager::put_mutex_guard(mutex_addr, guard).map_err(SleepError::Generic)?;

    Ok(())
}
