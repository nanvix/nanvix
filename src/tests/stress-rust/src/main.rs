// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::as_conversions)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

mod tests;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::unistd::STDOUT_FILENO;
use ::syscall::unistd;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Runs all stress workloads and emits a sentinel when they finish successfully.
///
/// # Returns
///
/// `Ok(())` on success or an error if a workload or the sentinel write fails.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    tests::run_all()?;

    {
        // Emit CI sentinel once all stress workloads complete.
        let sentinel: &[u8] = b"ok";
        unistd::write(STDOUT_FILENO, sentinel)?;
    }

    Ok(())
}
