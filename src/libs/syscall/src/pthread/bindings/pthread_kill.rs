// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::ErrorCode,
    pm::ThreadIdentifier,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sends a signal to a thread in the calling process. A caught signal runs in the target thread;
/// termination, stop, and continue dispositions affect the whole process. Blocked signals remain
/// pending on the target thread until it unblocks them.
///
/// # Parameters
///
/// - `thread`: Identifier of the target thread.
/// - `sig`: Signal number, or zero to check the target without sending a signal.
///
/// # Returns
///
/// Zero on success, or a positive error number on failure. Does not set `errno`.
///
/// # Errors
///
/// Returns `EINVAL` for an invalid signal, or `ESRCH` if the target is not a live thread in the
/// calling process.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub extern "C" fn pthread_kill(thread: pthread_t, sig: c_int) -> c_int {
    let tid: ThreadIdentifier = match ThreadIdentifier::try_from(thread) {
        Ok(tid) => tid,
        Err(error) => {
            ::syslog::error!("pthread_kill(): invalid thread (thread={thread}, error={error:?})");
            return ErrorCode::NoSuchProcess.get();
        },
    };
    match ::sys::kcall::pm::__kcall_kill_thread(tid, sig) {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}
