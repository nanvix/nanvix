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
/// Gets the based address for the user-space thread data area of a thread.
///
/// # Parameters
///
/// - `pid`: The process identifier of the calling process.
/// - `tid`: The thread identifier of the calling thread.
///
/// # Return Value
///
/// On successful completion, this function returns the thread data area pointer. On failure,
/// this function returns an error code that indicates the reason of failure.
///
/// # Errors
///
/// This function fails with the following error codes:
///
/// - [`ErrorCode::NoSuchEntry`]: The specified process or thread does not exist.
/// - [`ErrorCode::ResourceBusy`]: The process manager is busy and cannot handle the request.
///
pub fn get_thread_data_area(pid: ProcessIdentifier, tid: ThreadIdentifier) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };

    trace!("pid={pid:?}, tid={tid:?}");

    // Handle kernel call.
    match pm.get_thread_data_area(pid, tid) {
        Ok(user_tda_opt) => {
            let user_tda_value: i64 = match user_tda_opt {
                Some(user_tda) => (usize::from(user_tda) as u32).into(),
                None => 0,
            };
            trace!("user_tda={user_tda_value:#x}");
            KcallResult::Success(user_tda_value.into())
        },

        Err(error) => {
            error!("{error:?}");
            KcallResult::Error(error.code.into())
        },
    }
}
