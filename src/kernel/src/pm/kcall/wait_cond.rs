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
    InterruptReason,
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
/// Waits on a condition variable. After releasing the mutex, restores ownership before returning
/// to the caller, including on timeout, interruption, or condition-variable allocation failure.
/// Process termination aborts the wait without waiting for a potentially terminated mutex owner.
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
    // The kernel-call ABI passes the timeout fields as 32-bit values, so the "no timeout" sentinel
    // is `u32::MAX` in each field (zero-extended into `usize` here). Comparing against `usize::MAX`
    // would only match on 32-bit targets; use the 32-bit sentinel so it works on x86_64 too.
    const NO_TIMEOUT: usize = u32::MAX as usize;
    let alarm: Option<SystemTime> = if timeout_s == NO_TIMEOUT && timeout_ns == NO_TIMEOUT {
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

    let guard: MutexGuard =
        ProcessManager::take_mutex_guard(pid, tid, mutex_addr).map_err(SleepError::Generic)?;
    // Pin the existing map entry before unlocking so reacquisition cannot allocate or change mutexes.
    let mutex: Mutex = match ProcessManager::get_mutex(mutex_addr) {
        Ok(mutex) => mutex,
        Err(error) => {
            ProcessManager::put_mutex_guard(mutex_addr, guard).map_err(SleepError::Generic)?;
            return Err(SleepError::Generic(error));
        },
    };
    drop(guard);

    let mut result: Result<(), SleepError> = match ProcessManager::get_cond(cond_addr) {
        Ok(cond) => {
            let result: Result<(), SleepError> = cond.wait(alarm);
            drop(cond);
            result.and(ProcessManager::put_cond(cond_addr).map_err(SleepError::Generic))
        },
        Err(error) => {
            error!(
                "wait_cond(): failed to get condition variable (pid={:?}, tid={:?}, \
                 cond_addr={:x?}, mutex_addr={:x?}, error={:?})",
                pid, tid, cond_addr, mutex_addr, error
            );
            Err(SleepError::Generic(error))
        },
    };

    if matches!(result, Err(SleepError::Interrupted(InterruptReason::Killed))) {
        return result.and(ProcessManager::put_mutex(mutex_addr).map_err(SleepError::Generic));
    }

    // Defer nonfatal errors until ownership is restored; do not reuse the condition wait's deadline.
    let guard: MutexGuard = loop {
        match mutex.lock(None) {
            Ok(guard) => break guard,
            Err(error @ SleepError::Interrupted(InterruptReason::Killed)) => {
                result = Err(error);
                return result
                    .and(ProcessManager::put_mutex(mutex_addr).map_err(SleepError::Generic));
            },
            Err(error) => {
                warn!("wait_cond(): retrying mutex reacquisition (error={error:?})");
                result = result.and(Err(error));
            },
        }
    };
    ProcessManager::put_mutex_guard(mutex_addr, guard).map_err(SleepError::Generic)?;

    result
}
