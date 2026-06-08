// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

mod file_system;
mod identity;

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
    // Run tests.
    identity::test();
    file_system::test();

    // Magic string.
    {
        let magic_string: &[u8] = "ok".as_bytes();
        unistd::write(STDOUT_FILENO, magic_string)?;
    }

    Ok(())
}
