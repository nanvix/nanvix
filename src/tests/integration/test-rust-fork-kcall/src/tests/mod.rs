// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod bulk_unaligned;
mod fork;
#[cfg(target_arch = "aarch64")]
mod fp_state;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every `fork()` test.
pub fn run_all() -> Result<(), Error> {
    fork::run()?;
    #[cfg(target_arch = "aarch64")]
    fp_state::run()?;
    bulk_unaligned::run()?;
    Ok(())
}
