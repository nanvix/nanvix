// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod bulk_unaligned;
mod fork;
mod peer_exit;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every `fork()` test.
pub fn run_all() -> Result<(), Error> {
    fork::run()?;
    bulk_unaligned::run()?;
    peer_exit::run()?;
    Ok(())
}
