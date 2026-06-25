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
        process::state::signal::{
            SignalDisposition,
            SignalHandler,
        },
        ProcessManager,
    },
};
use ::core::mem::size_of;
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
    pm::{
        ProcessIdentifier,
        SigAction,
        SIGKILL,
        SIGSTOP,
        SIG_DFL,
        SIG_IGN,
        SIG_MAX,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for `sigaction()`, which gets and/or sets the disposition of a signal for
/// the calling process.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Signal number.
/// - `arg1`: User-space pointer to the new action, or null to leave the disposition unchanged.
/// - `arg2`: User-space pointer that receives the previous action, or null if not wanted.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn sigaction(caller_pid: ProcessIdentifier, arg0: u32, arg1: u32, arg2: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Validate the signal number. Out-of-range values (including would-be negative `c_int`s, which
    // arrive here as large unsigned values) are rejected.
    let signum: usize = match usize::try_from(arg0) {
        Ok(signum) if (1..=SIG_MAX).contains(&signum) => signum,
        _ => {
            let reason: &str = "invalid signal number";
            error!("{reason} (signum={arg0})");
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        },
    };

    let act_ptr: usize = arg1 as usize;
    let oldact_ptr: usize = arg2 as usize;

    // Check if the old action buffer lies in user space, if provided. Do this before mutating the
    // disposition so an invalid output pointer cannot turn the call into a partial success.
    if oldact_ptr != 0 {
        let oldact_addr: VirtualAddress = VirtualAddress::from_raw_value(oldact_ptr);
        if !Vmem::is_user_region(oldact_addr, size_of::<SigAction>()) {
            let reason: &str = "old action buffer does not lie in user space";
            error!("{reason} (oldact={oldact_addr:?})");
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        }
    }

    // Build the new disposition from the user-supplied action, if one was provided.
    let new: Option<SignalDisposition> = if act_ptr != 0 {
        // The disposition of SIGKILL and SIGSTOP can never be changed.
        if signum == SIGKILL || signum == SIGSTOP {
            let reason: &str = "cannot change the disposition of SIGKILL or SIGSTOP";
            error!("{reason} (signum={signum})");
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        }

        // Check if the new action lies in user space.
        let act_addr: VirtualAddress = VirtualAddress::from_raw_value(act_ptr);
        if !Vmem::is_user_region(act_addr, size_of::<SigAction>()) {
            let reason: &str = "new action does not lie in user space";
            error!("{reason} (act={act_addr:?})");
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        }

        // Copy the new action from user space.
        let mut act: SigAction = SigAction::default();
        if let Err(error) =
            pm::copy_from_user(pm, caller_pid, &mut act, act_ptr as *const SigAction)
        {
            error!("failed to copy new action from user space (error={error:?})");
            return KcallResult::Error(error.code.into());
        }

        // Map the handler field to a disposition.
        match act.sa_handler {
            SIG_DFL => Some(SignalDisposition::Default),
            SIG_IGN => Some(SignalDisposition::Ignore),
            entry => {
                // A caught signal must point to a handler in user space.
                let entry_addr: VirtualAddress = VirtualAddress::from_raw_value(entry);
                if !Vmem::is_user_addr(entry_addr) {
                    let reason: &str = "signal handler does not lie in user space";
                    error!("{reason} (handler={entry_addr:?})");
                    return KcallResult::Error(ErrorCode::InvalidArgument.into());
                }
                let handler = match crate::mm::try_box(SignalHandler {
                    entry: entry_addr,
                    mask: act.sa_mask,
                    flags: act.sa_flags,
                    sigaction: act.sa_sigaction,
                }) {
                    Ok(handler) => handler,
                    Err(error) => {
                        error!("{error:?}");
                        return KcallResult::Error(error.code.into());
                    },
                };
                Some(SignalDisposition::Handler(handler))
            },
        }
    } else {
        None
    };

    // Swap the disposition, capturing the previous one for the `oldact` return.
    let old: SigAction = match pm.sigaction(caller_pid, signum, new) {
        Ok(old) => old,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };

    // Report the previous action through `oldact`, if requested.
    if oldact_ptr != 0 {
        if let Err(error) = pm::copy_to_user(pm, caller_pid, oldact_ptr as *mut SigAction, &old) {
            error!("failed to copy old action to user space (error={error:?})");
            return KcallResult::Error(error.code.into());
        }
    }

    KcallResult::ok()
}
