// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

mod clock;
mod file_system;
mod identity;
mod network;

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::sys::error::Error;
use ::posix::{
    sys::types::size_t,
    unistd,
    venv,
    venv::VirtualEnvironmentIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    let env: VirtualEnvironmentIdentifier = venv::join(VirtualEnvironmentIdentifier::NEW)?;
    ::nvx::log!("joined environment {:?}", env);

    // Run tests.
    clock::test();
    identity::test();
    file_system::test();
    if let Err(error) = network::test_network() {
        ::nvx::log!("network test failed: {:?}", error);
    }

    venv::leave(env)?;
    ::nvx::log!("left environment {:?}", env);

    // Magic string.
    {
        let magic_string: &[u8] = "ok".as_bytes();
        unistd::write(unistd::STDOUT_FILENO, magic_string.as_ptr(), magic_string.len() as size_t);
    }

    Ok(())
}
