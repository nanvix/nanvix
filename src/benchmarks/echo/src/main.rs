// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::sys::error::Error;
use ::posix::{
    sys::types::{
        size_t,
        ssize_t,
    },
    unistd,
};

//==================================================================================================
// Constants
//==================================================================================================

const MAX_REQUEST_SIZE: usize = 32;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    let stdin: i32 = unistd::STDIN_FILENO;
    let stdout: i32 = unistd::STDOUT_FILENO;
    let mut buffer: [u8; MAX_REQUEST_SIZE] = [0; MAX_REQUEST_SIZE];

    let n: ssize_t = match unistd::read(stdin, buffer.as_mut_ptr(), buffer.len() as size_t) {
        n if n >= 0 => n,
        _ => 0,
    };

    if n > 0 {
        unistd::write(stdout, buffer.as_ptr(), n as size_t);
    }

    Ok(())
}
