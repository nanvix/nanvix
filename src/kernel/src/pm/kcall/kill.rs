// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    pm::{
        process::state::signal::KillOutcome,
        ProcessManager,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::ProcessIdentifier,
    ExitStatus,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for `kill()`, which posts a signal to a target process.
///
/// The heavy lifting is performed by [`ProcessManager::kill`], which enforces the privilege policy,
/// posts the signal, and evaluates its default action. A self-directed fatal signal cannot be
/// completed there because terminating the calling process requires `exit()` (which performs a
/// context switch and never returns); that case is reported as [`KillOutcome::TerminateSelf`] and
/// finished here.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Encoded process identifier of the target process.
/// - `arg1`: Signal number to post, or zero for the null-signal probe.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn kill(caller_pid: ProcessIdentifier, arg0: u32, arg1: u32) -> KcallResult {
    // Unpack the target process identifier.
    let target: ProcessIdentifier = match ProcessIdentifier::try_from(arg0) {
        Ok(target) => target,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };

    // The signal number is range-checked by the posting primitive.
    let signum: usize = arg1 as usize;

    // Post the signal and evaluate its delivery. The borrow of the process manager is released
    // before the self-termination path below, which re-acquires it.
    let outcome: Result<KillOutcome, Error> = {
        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        pm.kill(caller_pid, target, signum)
    };

    match outcome {
        Ok(KillOutcome::Done) => KcallResult::ok(),
        Ok(KillOutcome::TerminateSelf) => {
            // The default action terminates the calling process. Reuse the self-exit primitive,
            // mirroring the exit status used by the cross-process termination path. On success it
            // performs a context switch and never returns, so only an error can surface here.
            // SAFETY: the calling process is not the kernel and no borrow of the process manager is
            // held at this point.
            match unsafe { ProcessManager::exit(ExitStatus::from(ErrorCode::Interrupted)) } {
                // The success variant carries the never type: a successful exit() switches context
                // and does not return, so this arm is unreachable. Matching it explicitly (instead
                // of unwrap_err()) keeps the path panic-free and forces a compile-time review if
                // exit()'s signature ever changes.
                Ok(never) => never,
                Err(e) => KcallResult::Error(e.code.into()),
            }
        },
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
