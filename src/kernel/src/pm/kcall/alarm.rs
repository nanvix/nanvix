// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::{
    kcall::KcallResult,
    pm::ProcessManager,
};
use ::sys::pm::ProcessIdentifier;

///
/// # Description
///
/// Replaces the calling process's SIGALRM timer.
///
/// # Parameters
///
/// - `pid`: Calling process.
/// - `seconds`: Delay in seconds, or zero to cancel.
///
/// # Returns
///
/// The previous timer's remaining seconds, rounded up, or a kernel error.
///
pub fn alarm(pid: ProcessIdentifier, seconds: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and kernel access is synchronized.
    match unsafe { ProcessManager::get_mut() }.alarm(pid, seconds) {
        Ok(remaining) => KcallResult::Success(remaining.into()),
        Err(error) => {
            error!("alarm(): failed (pid={pid:?}, seconds={seconds}, error={error:?})");
            KcallResult::Error(error.code.into())
        },
    }
}
