// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    pm::ProcessManager,
};
use ::sys::pm::{
    ProcessIdentifier,
    ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Detaches a thread. A detached thread is automatically harvested when it exits, without
/// requiring another thread to join it.
///
/// # Parameters
///
/// - `pid`: Process identifier of the calling process.
/// - `arg0`: Thread identifier of the thread to detach.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is safe to call if and only if the following conditions are met:
/// - The process manager is initialized.
/// - Access to the process manager is synchronized.
/// - The memory manager is initialized.
/// - Access to the memory manager is synchronized.
///
pub fn detach_thread(pid: ProcessIdentifier, arg0: u32) -> KcallResult {
    let tid: ThreadIdentifier = match ThreadIdentifier::try_from(arg0) {
        Ok(tid) => tid,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };

    // SAFETY: the process manager and virtual memory manager are initialized and access is
    // synchronized.
    match unsafe { ProcessManager::detach_thread(pid, tid) } {
        Ok(()) => KcallResult::ok(),
        Err(error) => {
            error!("detach_thread(): failed (pid={pid:?}, tid={tid:?}, error={error:?})");
            KcallResult::Error(error.code.into())
        },
    }
}
