// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    self,
    ProcessManager,
    SleepError,
};
use ::sys::{
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    ExitStatus,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Joins a thread.
///
/// # Parameters
///
/// - `pid`: Process identifier in which the thread is running.
/// - `arg0`: Thread identifier of the thread to join.
/// - `arg1`: Store location for the return value of the thread.
///
/// # Returns
///
/// Upon successful completion, the status of the thread is returned. Otherwise, an error code is
/// returned instead.
///
/// # Safety
///
/// This function panics if the kernel process tries to sleep.
///
/// This function is unsafe because it blocks the calling thread until it is woken up by another
/// thread.
///
/// This function is safe to use if and only if the following conditions are met:
///
/// - The calling process is not the kernel process.
/// - This function is invoked without holding any resources.
/// - The process manager is initialized.
/// - Access to the process manager is synchronized.
/// - The memory manager is initialized.
/// - Access to the memory manager is synchronized.
///
pub unsafe fn join_thread(
    pid: ProcessIdentifier,
    arg0: u32,
    arg1: u32,
) -> Result<ExitStatus, SleepError> {
    // Unpack kernel call arguments.
    let tid: ThreadIdentifier = ThreadIdentifier::from(arg0 as usize);
    let retval: *mut ExitStatus = arg1 as *mut ExitStatus;

    let status: ExitStatus = ProcessManager::join_thread(pid, tid)?;

    pm::copy_to_user::<ExitStatus>(ProcessManager::get_mut(), pid, retval, &status)
        .map_err(SleepError::Generic)?;

    Ok(ExitStatus::ok())
}
