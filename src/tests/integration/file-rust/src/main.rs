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

/// Must come first.
extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

mod safe;
mod r#unsafe;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::syscall::unistd;
use sysapi::unistd::STDOUT_FILENO;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    r#unsafe::test();
    safe::test();

    // Magic string.
    {
        let magic_string: &[u8] = "ok".as_bytes();
        unistd::write(STDOUT_FILENO, magic_string)?;
    }

    Ok(())
}
