// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    pm::ProcessManager,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Terminates a target process on behalf of a calling process.
///
/// # Parameters
///
/// - `pm`: Reference to the process manager.
/// - `caller_pid`: Identifier of the calling process.
/// - `pid`: Identifier of the process to terminate.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
fn do_terminate(
    pm: &mut ProcessManager,
    caller_pid: ProcessIdentifier,
    pid: ProcessIdentifier,
) -> Result<(), Error> {
    trace!("caller_pid={:?}, pid={:?}", caller_pid, pid);

    // Check if the calling process has process-management capabilities.
    if !pm.has_capability(caller_pid, Capability::ProcessManagement)? {
        let reason: &str = "process does not have process management capability";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    pm.terminate(pid)
}

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

    // Unpack kernel call arguments.
    let pid: ProcessIdentifier = match ProcessIdentifier::try_from(arg0) {
        Ok(pid) => pid,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };
    match do_terminate(pm, caller_pid, pid) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
