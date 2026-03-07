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
    pm::ProcessIdentifier,
    signal::{
        SignalAction,
        SIG_DFL,
        SIG_IGN,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for `sigaction()`. Registers or queries a signal handler for the calling
/// process.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Signal number (1-based).
/// - `arg1`: New handler address (`SIG_DFL`, `SIG_IGN`, or a user-space function pointer). If
///   `u32::MAX`, the call is a query and the handler is not changed.
/// - `arg2`: Signal flags.
///
/// # Returns
///
/// On success, returns the previous handler address as a [`KcallResult::Success`].
/// On failure, returns the error code.
///
pub fn do_sigaction(
    caller_pid: ProcessIdentifier,
    arg0: u32,
    arg1: u32,
    arg2: u32,
) -> KcallResult {
    let _: ProcessIdentifier = caller_pid;

    let signum: i32 = arg0 as i32;
    let new_handler: u32 = arg1;
    let flags: u32 = arg2;

    // Query-only: return the current handler without modifying it.
    if new_handler == u32::MAX {
        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        match pm.get_running_process_state_mut().get_signal_action(signum) {
            Ok(action) => {
                let h: u32 = action.handler() as u32;
                KcallResult::Success(h.into())
            },
            Err(e) => {
                error!("{e:?}");
                KcallResult::Error(e.code.into())
            },
        }
    } else {
        let handler_usize: usize = match new_handler {
            0 => SIG_DFL,
            1 => SIG_IGN,
            addr => addr as usize,
        };

        let action: SignalAction = SignalAction::new(handler_usize, 0, flags);

        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        match pm.get_running_process_state_mut().set_signal_action(signum, action) {
            Ok(old) => {
                let h: u32 = old.handler() as u32;
                KcallResult::Success(h.into())
            },
            Err(e) => {
                error!("{e:?}");
                KcallResult::Error(e.code.into())
            },
        }
    }
}
