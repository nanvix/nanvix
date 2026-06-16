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

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    tests::run_all()?;

    // Magic string for CI harness.
    {
        let magic_string: &[u8] = b"ok";
        unistd::write(STDOUT_FILENO, magic_string)?;
    }

    Ok(())
}
