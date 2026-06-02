// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::sys::{
    error::Error,
    kcall::debug,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let msg: &str = "Hello, world from Rust!\n";
    let _ = debug::__kcall_debug(msg.as_ptr(), msg.len());

    Ok(())
}
