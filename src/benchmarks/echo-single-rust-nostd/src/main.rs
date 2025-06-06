// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate nvx;

use ::sys::error::Error;
use ::syscall::{
    sys::types::ssize_t,
    unistd,
};

//==================================================================================================
// Constants
//==================================================================================================

const MAX_REQUEST_SIZE: usize = 4096;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let stdin: i32 = unistd::STDIN_FILENO;
    let stdout: i32 = unistd::STDOUT_FILENO;
    let mut buffer: [u8; MAX_REQUEST_SIZE] = [0; MAX_REQUEST_SIZE];
    let mut n: usize = 0;

    let nread: ssize_t = match unistd::read(stdin, &mut buffer[n..]) {
        // Error encountered.
        Err(_error) => 0,
        // Read some bytes.
        Ok(n) => n as ssize_t,
    };
    n += nread as usize;

    if n > 0 {
        unistd::write(stdout, &buffer[..n])?;
    }

    Ok(())
}
