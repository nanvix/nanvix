// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::mutex::{
        Mutex,
        MutexGuard,
    },
    ProcessManager,
    SleepError,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        ConditionAddress,
        MutexAddress,
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::SystemTime,
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
/// - `timeout_s`: Timeout in seconds.
/// - `timeout_ns`: Timeout in nanoseconds.
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
    timeout_s: usize,
    timeout_ns: usize,
) -> Result<(), SleepError> {
    trace!(
        "wait_cond(): pid={:?}, tid={:?}, cond_addr={:#x?}, mutex_addr={:#x?}, timeout_s={:?}, \
         timeout_ns={:?}",
        pid,
        tid,
        cond_addr,
        mutex_addr,
        timeout_s,
        timeout_ns
    );

    // Unpack kernel call arguments.
    let cond_addr: ConditionAddress = ConditionAddress::from(cond_addr);
    let mutex_addr: MutexAddress = MutexAddress::from(mutex_addr);
    let alarm: Option<SystemTime> = if timeout_s == u32::MAX as usize
        && timeout_ns == u32::MAX as usize
    {
        None
    } else {
        match SystemTime::new(timeout_s as u64, timeout_ns as u32) {
            Some(timeout) => Some(timeout),
            None => {
                let reason: &str = "invalid timeout";
                error!(
                    "wait_cond(): {} (pid={:?}, tid={:?}, cond_addr={:#x?}, mutex_addr={:#x?}, \
                     timeout_s={:?}, timeout_ns={:?})",
                    reason, pid, tid, cond_addr, mutex_addr, timeout_s, timeout_ns
                );
                return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
            },
        }
    };

    {
        ProcessManager::take_mutex_guard(pid, tid, mutex_addr).map_err(SleepError::Generic)?;
        // The mutex guard is dropped, causing threads to be notified.
    }

    let result: Result<(), SleepError> = {
        match ProcessManager::get_cond(cond_addr) {
            Ok(cond) => cond.wait(alarm),
            Err(error) => {
                error!(
                    "wait_cond(): failed to get condition variable (pid={:?}, tid={:?}, \
                     cond_addr={:x?}, mutex_addr={:x?}, error={:?})",
                    pid, tid, cond_addr, mutex_addr, error
                );
                Err(SleepError::Generic(error))
            },
        }
        // The condition variable is dropped, causing its reference count to decrease.
    };
    ProcessManager::put_cond(cond_addr).map_err(SleepError::Generic)?;

    // Reacquire the mutex.
    let mutex: Mutex = ProcessManager::get_mutex(mutex_addr).map_err(SleepError::Generic)?;
    let guard: MutexGuard = mutex.lock(None)?;
    ProcessManager::put_mutex_guard(mutex_addr, guard).map_err(SleepError::Generic)?;

    result
}
