// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::{
        KcallArgs,
        KcallResult,
    },
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
/// - `pm`: A reference to the process manager.
/// - `args`: The kernel call arguments.
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
pub fn get_thread_data_area(pm: &ProcessManager, args: &KcallArgs) -> KcallResult {
    // Unpack arguments.
    let pid: ProcessIdentifier = args.pid;
    let tid: ThreadIdentifier = args.tid;

    trace!("get_thread_data_area(): pid={pid:?}, tid={tid:?}");

    // Handle kernel call.
    match pm.get_thread_data_area(pid, tid) {
        Ok(user_tda_opt) => {
            let user_tda_value: i64 = match user_tda_opt {
                Some(user_tda) => u32::from(user_tda).into(),
                None => 0,
            };
            trace!("get_thread_data_area(): success (user_tda={user_tda_value:#x})");
            KcallResult::Success(user_tda_value.into())
        },

        Err(error) => {
            error!("get_thread_data_area(): failed: {error:?}");
            KcallResult::Error(error.code.into())
        },
    }
}
