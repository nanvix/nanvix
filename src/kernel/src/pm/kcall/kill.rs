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
    error::ErrorCode,
    pm::ProcessIdentifier,
    signal,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for `kill()`. Sends a signal to the main thread of the target process.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Target process identifier (encoded as `u32`).
/// - `arg1`: Signal number to send.
///
/// # Returns
///
/// On success, returns [`KcallResult::ok()`]. On failure, returns the error code.
///
pub fn do_kill(caller_pid: ProcessIdentifier, arg0: u32, arg1: u32) -> KcallResult {
    let _: ProcessIdentifier = caller_pid;

    let target_pid: ProcessIdentifier = match ProcessIdentifier::try_from(arg0) {
        Ok(pid) => pid,
        Err(e) => {
            error!("{e:?}");
            return KcallResult::Error(e.code.into());
        },
    };

    let signum: i32 = arg1 as i32;

    if !signal::is_valid_signal(signum) {
        let reason: &'static str = "invalid signal number";
        error!("{:?} (signum={})", reason, signum);
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    match pm.queue_signal(target_pid, signum) {
        Ok(()) => KcallResult::ok(),
        Err(e) => {
            error!("{e:?}");
            KcallResult::Error(e.code.into())
        },
    }
}
