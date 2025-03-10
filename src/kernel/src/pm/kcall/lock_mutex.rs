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
use ::sys::pm::MutexAddress;

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
/// - `raw_addr`: Address of the mutex to lock.
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
pub unsafe fn lock_mutex(raw_addr: usize) -> Result<(), SleepError> {
    // Unpack kernel call arguments.
    let mutex_addr: MutexAddress = MutexAddress::from(raw_addr);

    let mutex: Mutex = ProcessManager::get_mutex(mutex_addr).map_err(SleepError::Generic)?;
    let guard: MutexGuard = mutex.lock()?;
    ProcessManager::put_mutex_guard(mutex_addr, guard).map_err(SleepError::Generic)
}
