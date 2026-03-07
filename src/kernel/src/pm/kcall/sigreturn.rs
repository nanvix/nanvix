// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::kcall::KcallResult;
use ::sys::pm::ProcessIdentifier;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for `sigreturn()`. Called from the sigreturn trampoline when a signal
/// handler returns. In the current minimal implementation this is a no-op because context
/// restoration is handled by the trampoline returning through the normal kcall path.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
///
/// # Returns
///
/// Always returns [`KcallResult::ok()`].
///
/// # Note
///
/// A full implementation would restore the saved pre-signal context (registers, signal mask, etc.)
/// from kernel memory. The current identity-mapping design allows a simpler approach where the
/// trampoline on the user stack directly invokes the sigreturn kcall and the kernel simply
/// acknowledges it.
///
pub fn do_sigreturn(caller_pid: ProcessIdentifier) -> KcallResult {
    let _: ProcessIdentifier = caller_pid;

    // Mark the thread as no longer in a signal handler.
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut crate::pm::ProcessManager = unsafe { crate::pm::ProcessManager::get_mut() };
    pm.clear_signal_handler_flag();

    KcallResult::ok()
}
