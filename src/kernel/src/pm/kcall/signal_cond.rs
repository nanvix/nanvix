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

pub unsafe fn signal_cond(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    cond_addr: usize,
    broadcast: bool,
) -> Result<usize, Error> {
    trace!(
        "signal_cond(): pid={pid:?}, tid={tid:?}, cond_addr={cond_addr:x?}, broadcast={broadcast}"
    );

    // Unpack kernel call arguments.
    let cond_addr: ConditionAddress = ConditionAddress::from(cond_addr);

    let awakened: usize = {
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
