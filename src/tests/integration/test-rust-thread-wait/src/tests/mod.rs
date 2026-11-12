// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod support;
mod thread_wait;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every thread-termination wait test.
pub fn run_all() -> Result<(), Error> {
    thread_wait::run()?;
    Ok(())
}
