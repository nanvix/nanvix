// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::{
        KcallResult,
        KcallSuccess,
    },
    pm::{
        ProcessManager,
        SigReturnFailure,
    },
};
use ::sys::{
    error::ErrorCode,
    ExitStatus,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for `sigreturn()`, which restores the calling thread's context after a
/// signal handler returns.
///
/// The heavy lifting is performed by [`ProcessManager::sigreturn_restore`], which validates the
/// on-stack signal frame, sanitizes its privileged state, and restores the interrupted context,
/// FPU state, and blocked mask. The saved accumulator is returned verbatim so the interrupted
/// kernel call's result survives the handler; this deliberately bypasses the dispatcher epilogue
/// that would otherwise overwrite the restored return register.
///
/// # Returns
///
/// On success, a [`KcallResult`] carrying the restored accumulator. A corrupt or forged frame
/// terminates the process via its default action.
///
pub fn sigreturn() -> KcallResult {
    // Restore the interrupted context. The borrow of the process manager is released before the
    // termination path below, which re-acquires it.
    let outcome: Result<i64, SigReturnFailure> = {
        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        pm.sigreturn_restore()
    };

    match outcome {
        Ok(accumulator) => KcallResult::Success(KcallSuccess::from(accumulator)),
        Err(SigReturnFailure::Forged) => {
            // A corrupt or forged frame cannot be trusted to resume safely: terminate the process
            // via its default action. On success `exit()` switches context and never returns, so
            // only an error can surface here.
            // SAFETY: the calling process is not the kernel and no borrow of the process manager is
            // held at this point.
            match unsafe { ProcessManager::exit(ExitStatus::from(ErrorCode::Interrupted)) } {
                Ok(never) => never,
                Err(error) => KcallResult::Error(error.code.into()),
            }
        },
        Err(SigReturnFailure::Unsupported) => KcallResult::Error(ErrorCode::InvalidSysCall.into()),
    }
}
