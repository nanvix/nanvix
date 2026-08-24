// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    mm::Vmem,
    pm::{
        self,
        ProcessManager,
    },
};
use ::core::mem::size_of;
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
    pm::{
        ProcessIdentifier,
        SigSet,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for `sigpending()`, which reports the set of signals that are pending on the
/// calling process but blocked from delivery to the calling thread (`pending & blocked`).
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process (owner of the output buffer).
/// - `caller_tid`: Identifier of the calling thread (owner of the blocked mask).
/// - `arg0`: User-space pointer that receives the pending-but-blocked signal set.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn sigpending(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    arg0: u32,
) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // The output buffer is mandatory and must lie in user space.
    let set_ptr: usize = arg0 as usize;
    if set_ptr == 0 {
        let reason: &str = "null pending signal set buffer";
        error!("{reason}");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }
    let set_addr: VirtualAddress = VirtualAddress::from_raw_value(set_ptr);
    if !Vmem::is_user_region(set_addr, size_of::<SigSet>()) {
        let reason: &str = "pending signal set buffer does not lie in user space";
        error!("{reason} (set={set_addr:?})");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Compute the pending-but-blocked set for the calling thread.
    let pending: SigSet = match pm.sigpending(caller_pid, caller_tid) {
        Ok(pending) => pending,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };

    // Report it through the user-supplied buffer.
    if let Err(error) = pm::copy_to_user_addr(pm, caller_pid, set_addr, &pending) {
        error!("failed to copy pending signal set to user space (error={error:?})");
        return KcallResult::Error(error.code.into());
    }

    KcallResult::ok()
}
