// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;

mod clock;
mod file_system;
mod identity;

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::sys::error::Error;
use ::posix::{
    sys::types::size_t,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    // Run tests.
    clock::test();
    identity::test();
    file_system::test();

    // Magic string.
    {
        let magic_string: &[u8] = "ok".as_bytes();
        unistd::write(unistd::STDOUT_FILENO, magic_string.as_ptr(), magic_string.len() as size_t);
    }

    Ok(())
}
