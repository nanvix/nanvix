// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Yields the processor.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise an error code is returned instead.
///
pub fn sched_yield() -> Result<(), Error> {
    sys::kcall::sched::sched_yield()
}
