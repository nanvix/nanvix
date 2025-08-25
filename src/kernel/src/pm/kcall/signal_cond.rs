// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::condvar::Condvar,
    ProcessManager,
};
use ::sys::{
    error::Error,
    pm::{
        ConditionAddress,
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Signals a condition variable.
///
/// # Parameters
///
/// - `pid`: Target process identifier.
/// - `tid`: Target thread identifier.
/// - `cond_addr`: Address of the condition variable.
/// - `broadcast`: If `true`, wakes all waiting threads. Otherwise, wakes a single waiting thread.
///
/// # Returns
///
/// Upon successful completion, this function returns the number of threads that were awakened.
/// Otherwise, it returns an error object that specifying the reason of failure.
///
/// # Safety
///
/// This function is unsafe because it operates on global variables.
///
/// It is safe to call this function if and only if the following conditions are met:
///
/// - The calling process does not hold a reference to the process manager.
///
pub unsafe fn signal_cond(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    cond_addr: usize,
    broadcast: bool,
) -> Result<u32, Error> {
    trace!(
        "signal_cond(): pid={pid:?}, tid={tid:?}, cond_addr={cond_addr:x?}, broadcast={broadcast}"
    );

    // Unpack kernel call arguments.
    let cond_addr: ConditionAddress = ConditionAddress::from(cond_addr);

    let awakened: u32 = {
        let cond: Condvar = ProcessManager::get_cond(cond_addr)?;
        if broadcast {
            cond.notify_all()?
        } else {
            cond.notify_first()?
        }
        // The condition variable is dropped, causing its reference count to decrease.
    };
    ProcessManager::put_cond(cond_addr)?;

    Ok(awakened)
}
