// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::sys::error::Error;
use ::sysapi::unistd::STDOUT_FILENO;
use ::syscall::unistd;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Entry point of the target program loaded by the `execv()` test. It writes the success sentinel
/// to the standard output and exits, demonstrating that `execv()` replaced the calling image with
/// this distinct program.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let magic_string: &[u8] = "ok".as_bytes();
    unistd::write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
