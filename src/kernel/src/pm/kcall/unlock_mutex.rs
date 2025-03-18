// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::ProcessManager;
use ::sys::{
    error::Error,
    pm::{
        MutexAddress,
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Unlocks a mutex.
///
/// # Parameters
///
/// - `pid`: Process identifier.
/// - `tid`: Thread identifier.
/// - `arg0`: Address of the mutex to unlock.
///
/// # Return
///
/// Upon successful completion, zero is returned. Upon failure, a negative error code is returned
/// instead.
///
/// # Safety
///
/// This function is unsafe because it operates on global variables.
///
/// This function is safe to use if and only if the following conditions are met:
///
/// - The calling process does not hold a reference to the process manager.
///
pub unsafe fn unlock_mutex(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    arg0: usize,
) -> Result<(), Error> {
    // Unpack kernel call arguments.
    let mutex_addr: MutexAddress = MutexAddress::from(arg0);

    ProcessManager::take_mutex_guard(pid, tid, mutex_addr).unwrap();
    // The mutex guard is dropped, causing threads to be notified.

    Ok(())
}
