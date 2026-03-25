// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod env;
mod sysinfo;
mod time;
mod uid_gid;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every miscellaneous system call test.
pub fn run_all() -> Result<(), Error> {
    uid_gid::run()?;
    time::run()?;
    sysinfo::run()?;
    env::run()?;
    Ok(())
}
