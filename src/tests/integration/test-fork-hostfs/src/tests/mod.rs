// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod fork_hostfs;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every host-filesystem file-descriptor duplication test.
pub fn run_all() -> Result<(), Error> {
    fork_hostfs::run()?;
    Ok(())
}
