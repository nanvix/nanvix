// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    ProcessManager,
    SleepError,
};
use ::sys::{
    mm::VirtualAddress,
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
/// Upon successful completion, the thread's status is stored at `arg1` and success is returned.
/// Otherwise, an error is returned. A failed status copy leaves the thread available for joining.
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
    let tid: ThreadIdentifier = match ThreadIdentifier::try_from(arg0) {
        Ok(tid) => tid,
        Err(error) => {
            error!("{error:?}");
            return Err(SleepError::Generic(error));
        },
    };
    let retval: VirtualAddress = VirtualAddress::from_raw_value(arg1 as usize);

    ProcessManager::join_thread(pid, tid, retval)?;

    Ok(ExitStatus::ok())
}
