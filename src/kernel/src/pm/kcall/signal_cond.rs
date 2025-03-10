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
    pm::ConditionAddress,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub unsafe fn signal_cond(cond_addr: usize, broadcast: bool) -> Result<(), Error> {
    // Unpack kernel call arguments.
    let cond_addr: ConditionAddress = ConditionAddress::from(cond_addr);

    {
        let cond: Condvar = ProcessManager::get_cond(cond_addr)?;
        if broadcast {
            cond.notify_all()?;
        } else {
            cond.notify_first()?;
        }
        // The condition variable is dropped, causing its reference count to decrease.
    }
    ProcessManager::put_cond(cond_addr)?;

    Ok(())
}
