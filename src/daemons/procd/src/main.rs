// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate nvx;

extern crate alloc;

use ::proc::ProcessDaemon;
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let mut procd: ProcessDaemon = match ProcessDaemon::init() {
        Ok(procd) => procd,
        Err(e) => panic!("failed to initialize process manager daemon (error={:?})", e),
    };

    procd.run();
    procd.shutdown();

    Ok(())
}
