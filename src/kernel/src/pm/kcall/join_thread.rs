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
// `map_err` uses an explicit closure instead of the `SleepError::Generic`
// constructor as a bare function value, which the Verus frontend cannot lower.
#[allow(clippy::redundant_closure)]
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

    let status: ExitStatus = ProcessManager::join_thread(pid, tid)?;

    pm::copy_to_user_addr::<ExitStatus>(ProcessManager::get_mut(), pid, retval, &status)
        .map_err(|error| SleepError::Generic(error))?;

    Ok(ExitStatus::ok())
}
