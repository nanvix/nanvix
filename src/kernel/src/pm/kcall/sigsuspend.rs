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
        if let Err(error) = pm::copy_from_user_addr(pm, caller_pid, &mut value, mask_addr) {
            error!("failed to copy signal mask from user space (error={error:?})");
            return KcallResult::Error(error.code.into());
        }
        value
    };

    // Atomically install the temporary mask, saving the current one for restoration by sigreturn().
    // If the new mask makes an already-pending caught signal deliverable, do not sleep: return
    // EINTR immediately and let the dispatcher's return-to-user checkpoint deliver the handler.
    {
        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        match pm.install_sigsuspend_mask(caller_pid, caller_tid, mask) {
            Ok(true) => return KcallResult::Error(ErrorCode::Interrupted.into()),
            Ok(false) => {},
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
        // SAFETY: no borrow of the process manager is held, the calling thread is not the kernel,
        // access to the process manager is synchronized, and the processor runs with interrupts
        // disabled in privileged mode.
        match unsafe { ProcessManager::sleep(None) } {
            // Spurious or job-control wakeup: keep waiting.
            Ok(()) => continue,
            Err(SleepError::Interrupted(InterruptReason::Killed)) => {
                // The process is being terminated. Complete the termination via exit(), which
                // performs a context switch and never returns.
                // SAFETY: the calling process is not the kernel and no borrow is held.
                let error: Error =
                    unsafe { ProcessManager::exit(ErrorCode::Interrupted.into()).unwrap_err() };
                panic!("failed to exit() (error={error:?})");
            },
            // A caught signal (or, defensively, any other interruption) ends the suspension with
            // EINTR; the pre-suspend mask is reinstated by sigreturn() after the handler runs.
            Err(SleepError::Interrupted(_)) => {
                return KcallResult::Error(ErrorCode::Interrupted.into());
            },
            Err(SleepError::Generic(error)) => {
                error!("sigsuspend failed to sleep: {error:?}");
                // The suspension failed before any handler ran, so sigreturn() will not restore the
                // temporary mask. Reinstate the pre-suspend mask directly so the failed call leaves
                // the thread's mask unchanged, as POSIX requires.
                // SAFETY: no borrow of the process manager is held and access is synchronized.
                let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
                if let Err(restore_error) = pm.restore_sigsuspend_mask(caller_tid) {
                    error!(
                        "failed to restore signal mask after sigsuspend failure \
                         (error={restore_error:?})"
                    );
                }
                return KcallResult::Error(error.code.into());
            },
        }
    }
}
