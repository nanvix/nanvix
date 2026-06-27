// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    mm::Vmem,
    pm::ProcessManager,
};
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for registering the calling process's user-space signal-return trampoline
/// (restorer).
///
/// The kernel installs the restorer as the return address of every caught-signal handler frame it
/// builds, so that a handler returns into the trampoline, which then issues `sigreturn()`. Each
/// freshly loaded image re-registers its restorer after `execv()`.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Address of the restorer trampoline in the calling process's address space.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn sig_restorer(caller_pid: ProcessIdentifier, arg0: u32) -> KcallResult {
    let restorer: VirtualAddress = VirtualAddress::from_raw_value(arg0 as usize);

    // The restorer must lie in the user address space.
    if !Vmem::is_user_addr(restorer) {
        let reason: &str = "restorer does not lie in user space";
        error!("{reason} (restorer={restorer:?})");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
    match pm.set_signal_restorer(caller_pid, restorer) {
        Ok(()) => KcallResult::ok(),
        Err(error) => KcallResult::Error(error.code.into()),
    }
}
