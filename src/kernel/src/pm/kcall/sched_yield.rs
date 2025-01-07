// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::process::ProcessManager;
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_sched_yield() -> Result<(), Error> {
    match ProcessManager::switch() {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

///
/// # Description
///
/// Yields the processor.
///
/// # Returns
///
/// Upon successful completion, 0 is returned. Upon failure, a negative error code is returned
/// instead.
///
pub fn sched_yield() -> i32 {
    trace!("sched_yield()");
    match do_sched_yield() {
        Ok(()) => 0,
        Err(e) => e.code.into_errno(),
    }
}
