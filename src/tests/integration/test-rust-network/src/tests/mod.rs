// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod inet;
// Unix-domain sockets require linuxd; networkd only supports AF_INET sockets.
#[cfg(not(feature = "standalone"))]
mod unix;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every network socket test.
pub fn run_all() -> Result<(), Error> {
    inet::run()?;
    // Unix-domain sockets are only available through linuxd.
    #[cfg(not(feature = "standalone"))]
    unix::run()?;
    Ok(())
}
