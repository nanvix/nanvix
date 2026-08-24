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
// `map_err` uses explicit closures instead of the `SleepError::Generic`
// constructor as a bare function value, which the Verus frontend cannot lower.
#[allow(clippy::redundant_closure)]
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
    // The kernel-call ABI passes the timeout fields as 32-bit values, so the "no timeout" sentinel
    // is `u32::MAX` in each field (zero-extended into `usize` here). Comparing against `usize::MAX`
    // would only match on 32-bit targets; use the 32-bit sentinel so it works on x86_64 too.
    const NO_TIMEOUT: usize = u32::MAX as usize;
    let timeout: Option<SystemTime> = if timeout_s == NO_TIMEOUT && timeout_ns == NO_TIMEOUT {
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

    let mutex: Mutex =
        ProcessManager::get_mutex(mutex_addr).map_err(|error| SleepError::Generic(error))?;
    let guard: MutexGuard = mutex.lock(timeout)?;
    ProcessManager::put_mutex_guard(mutex_addr, guard).map_err(|error| SleepError::Generic(error))
}
