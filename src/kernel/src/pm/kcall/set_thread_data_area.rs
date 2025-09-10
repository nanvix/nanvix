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
    mm::Vmem,
    pm::ProcessManager,
};
use ::sys::pm::ProcessIdentifier;
use sys::{
    error::ErrorCode,
    mm::VirtualAddress,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the base address for the user-space thread data area of a thread.
///
/// # Parameters
///
/// - `pm`: A mutable reference to the process manager.
/// - `args`: The kernel call arguments.
///
/// # Return Value
///
/// On successful completion, this function returns a status code for a successful kernel call. On
/// failure, this function returns an error code that indicates the reason of failure.
///
/// # Errors
///
/// This function fails with the following error codes:
///
/// - [`ErrorCode::InvalidArgument`]: The provided thread data area pointer is invalid.
/// - [`ErrorCode::NoSuchEntry`]: The specified process or thread does not exist.
/// - [`ErrorCode::ResourceBusy`]: The process manager is busy and cannot handle the request.
///
pub fn set_thread_data_area(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Unpack arguments.
    let pid: ProcessIdentifier = args.pid;
    let tid: ThreadIdentifier = args.tid;
    let user_tda: VirtualAddress = VirtualAddress::from_raw_value(args.arg0 as usize);

    trace!("pid={pid:?}, tid={tid:?}, user_tda={user_tda:?}");

    // Check if tread-local storage does not lie within the user space.
    let user_tda: Option<VirtualAddress> = if user_tda != VirtualAddress::from_raw_value(0) {
        if !Vmem::is_user_addr(user_tda) {
            error!(
                "invalid base address for the user-space thread data area (tid={tid:?}, \
                 pid={pid:?}, user_tda={user_tda:?})"
            );
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        }

        Some(user_tda)
    } else {
        None
    };

    // Handle kernel call.
    match pm.set_thread_data_area(pid, tid, user_tda) {
        Ok(()) => {
            trace!("success");
            KcallResult::ok()
        },

        Err(error) => {
            error!("{error:?}");
            KcallResult::Error(error.code.into())
        },
    }
}
