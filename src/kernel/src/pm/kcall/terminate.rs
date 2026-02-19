// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    pm::ProcessManager,
};
use ::sys::pm::ProcessIdentifier;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for terminating a process.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Encoded process identifier of the process to terminate.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn terminate(caller_pid: ProcessIdentifier, arg0: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // TODO (#1434): check if calling process has enough privileges to terminate the target process.
    let _: ProcessIdentifier = caller_pid;

    // Unpack kernel call arguments.
    let pid: ProcessIdentifier = match ProcessIdentifier::try_from(arg0) {
        Ok(pid) => pid,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };
    match pm.terminate(pid) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
