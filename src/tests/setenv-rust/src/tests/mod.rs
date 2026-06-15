// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod fork;
mod setenv;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every environment-variable test (single process, then `fork()`-based isolation).
pub fn run_all() -> Result<(), Error> {
    setenv::run()?;
    fork::run()?;
    Ok(())
}
