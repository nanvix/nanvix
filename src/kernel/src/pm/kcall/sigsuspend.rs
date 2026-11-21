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
        process::state::signal::DefaultAction,
        InterruptReason,
        ProcessManager,
        SleepError,
    },
};
use ::core::mem::size_of;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
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

fn terminate_for_signal() -> KcallResult {
    // SAFETY: the caller is a user process, interrupts are disabled, and no manager borrow is held.
    match unsafe { ProcessManager::exit(ErrorCode::Interrupted.into()) } {
        Ok(never) => never,
        Err(error) => {
            error!("failed to terminate during sigsuspend (error={error:?})");
            KcallResult::Error(error.code.into())
        },
    }
}

fn fail_sigsuspend(tid: ThreadIdentifier, error: Error) -> KcallResult {
    error!("sigsuspend failed (error={error:?})");
    // SAFETY: access is synchronized and no other process-manager borrow is held.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
    if let Err(restore_error) = pm.restore_sigsuspend_mask(tid) {
        error!("failed to restore mask after sigsuspend failure (error={restore_error:?})");
    }
    KcallResult::Error(error.code.into())
}

///
/// # Description
///
/// Kernel call handler for `sigsuspend()`, which atomically replaces the calling thread's blocked
/// mask with `mask` and suspends the thread until a caught signal is delivered.
///
/// The previous mask is saved and reinstated by `sigreturn()` once the interrupting handler returns,
/// so the call leaves the mask unchanged. `sigsuspend()` always returns `EINTR`; it is never
/// restarted, so no restart record is left for the delivery checkpoint.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process (owner of the mask buffer).
/// - `caller_tid`: Identifier of the calling thread (owner of the blocked mask).
/// - `arg0`: User-space pointer to the temporary mask to install while suspended.
///
/// # Returns
///
/// A [`KcallResult`] carrying [`ErrorCode::Interrupted`] once a signal is caught, or another error
/// code on failure.
///
pub fn sigsuspend(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    arg0: u32,
) -> KcallResult {
    // The mask buffer is mandatory and must lie in user space.
    let mask_ptr: usize = arg0 as usize;
    if mask_ptr == 0 {
        let reason: &str = "null signal mask buffer";
        error!("{reason}");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }
    let mask_addr: VirtualAddress = VirtualAddress::from_raw_value(mask_ptr);
    if !Vmem::is_user_region(mask_addr, size_of::<SigSet>()) {
        let reason: &str = "signal mask buffer does not lie in user space";
        error!("{reason} (mask={mask_addr:?})");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Read the temporary mask from user space.
    let mask: SigSet = {
        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        let mut value: SigSet = 0;
        if let Err(error) =
            pm::copy_from_user(pm, caller_pid, &mut value, mask_ptr as *const SigSet)
        {
            error!("failed to copy signal mask from user space (error={error:?})");
            return KcallResult::Error(error.code.into());
        }
        value
    };

    // Atomically install the temporary mask, saving the current one for restoration by sigreturn().
    {
        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        match pm.install_sigsuspend_mask(caller_pid, caller_tid, mask) {
            Ok(()) => {},
            Err(error) => {
                error!("{error:?}");
                return KcallResult::Error(error.code.into());
            },
        }
    }

    // Block until a signal is delivered. A spurious or job-control wakeup re-suspends with the
    // temporary mask still installed; a caught signal interrupts the sleep and the call returns
    // EINTR (its mask reinstated by sigreturn() once the handler runs); a fatal signal tears the
    // process down.
    loop {
        // The temporary mask can make a previously blocked default action deliverable. A stop
        // yields without completing sigsuspend; after SIGCONT, keep waiting for a caught signal.
        let default: Option<DefaultAction> = {
            // SAFETY: access is synchronized and no other process-manager borrow is held.
            let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
            pm.deliver_pending_default_signal()
        };
        match default {
            Some(DefaultAction::Terminate | DefaultAction::Core) => return terminate_for_signal(),
            Some(DefaultAction::Stop) => {
                // SAFETY: no manager borrow is held; interrupts are disabled in privileged mode.
                if let Err(error) = unsafe { ProcessManager::giveup() } {
                    return fail_sigsuspend(caller_tid, error);
                }
                continue;
            },
            Some(DefaultAction::Ignore | DefaultAction::Continue) | None => {},
        }
        {
            // SAFETY: access is synchronized and no other process-manager borrow is held.
            let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
            match pm.has_pending_caught_signal(caller_pid, caller_tid) {
                Ok(true) => return KcallResult::Error(ErrorCode::Interrupted.into()),
                Ok(false) => {},
                Err(error) => return fail_sigsuspend(caller_tid, error),
            }
        }

        // SAFETY: no borrow of the process manager is held, the calling thread is not the kernel,
        // access to the process manager is synchronized, and the processor runs with interrupts
        // disabled in privileged mode.
        match unsafe { ProcessManager::sleep(None) } {
            // Spurious or job-control wakeup: keep waiting.
            Ok(()) => continue,
            Err(SleepError::Interrupted(InterruptReason::Killed)) => {
                return terminate_for_signal();
            },
            // Re-evaluate caught signals before ending the suspension with
            // EINTR; the pre-suspend mask is reinstated by sigreturn() after the handler runs.
            Err(SleepError::Interrupted(_)) => continue,
            Err(SleepError::Generic(error)) => return fail_sigsuspend(caller_tid, error),
        }
    }
}
