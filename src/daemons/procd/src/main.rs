// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::nvx::sys::error::Error;
use ::procd::ProcessDaemon;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    let mut procd: ProcessDaemon = match ProcessDaemon::init() {
        Ok(procd) => procd,
        Err(e) => panic!("failed to initialize process manager daemon (error={:?})", e),
    };

    procd.run();
    procd.shutdown();

    Ok(())
}
