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
/// Locks a mutex.
///
/// # Parameters
///
/// - `pid`: Identifier of the process that is locking the mutex.
/// - `tid`: Identifier of the thread that is locking the mutex.
/// - `mutex_addr`: Address of the mutex to lock.
/// - `timeout_s`: Timeout in seconds.
/// - `timeout_ns`: Timeout in nanoseconds.
///
/// # Return
///
/// Upon successful completion, zero is returned. Upon failure, a negative error code is returned
/// instead.
///
/// # Safety
///
/// This function is unsafe because:
/// - It operates on global variables
/// - It may panic.
/// - It may block the calling thread until it is woken up by another thread.
///
/// This function is safe to use if and only if the following conditions are met:
/// - The calling process is not the kernel process.
/// - This function is invoked without holding any resources.
/// - The calling process does not hold a reference to the process manager.
///
pub unsafe fn lock_mutex(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    mutex_addr: usize,
    timeout_s: usize,
    timeout_ns: usize,
) -> Result<(), SleepError> {
    trace!(
        "lock_mutex(): pid={pid:?}, tid={tid:?},  mutex_addr={mutex_addr:x?}, \
         timeout_s={timeout_s:?}, timeout_ns={timeout_ns:?}"
    );
    // Unpack kernel call arguments.
    let mutex_addr: MutexAddress = MutexAddress::from(mutex_addr);
    let timeout: Option<SystemTime> = if timeout_s == u32::MAX as usize
        && timeout_ns == u32::MAX as usize
    {
        None
    } else {
        match SystemTime::new(timeout_s as u64, timeout_ns as u32) {
            Some(timeout) => Some(timeout),
            None => {
                let reason: &str = "invalid timeout";
                error!(
                    "lock_mutex(): {} (mutex_addr={:x?}, timeout_s={:?}, timeout_ns={:?})",
                    reason, mutex_addr, timeout_s, timeout_ns
                );
                return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
            },
        }
    };

    let mutex: Mutex = ProcessManager::get_mutex(mutex_addr).map_err(SleepError::Generic)?;
    let guard: MutexGuard = mutex.lock(timeout)?;
    ProcessManager::put_mutex_guard(mutex_addr, guard).map_err(SleepError::Generic)
}
