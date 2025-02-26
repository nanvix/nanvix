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
use ::sys::pm::{
    ProcessIdentifier,
    ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn join_thread(arg0: u32, arg1: u32) -> Result<usize, SleepError> {
    // Unpack kernel call arguments.
    let tid: ThreadIdentifier = ThreadIdentifier::from(arg0 as usize);
    let retval: *mut usize = arg1 as *mut usize;
    let pid: ProcessIdentifier = ProcessManager::get_pid().map_err(SleepError::Generic)?;

    let status: usize = ProcessManager::join_thread(pid, tid)?;

    pm::copy_to_user::<usize>(pid, retval, &status).map_err(SleepError::Generic)?;

    Ok(0)
}
