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
use ::sysapi::{
    ffi::c_int,
    sys_types::c_ssize_t,
    unistd::{
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::syscall::unistd;

//==================================================================================================
// Constants
//==================================================================================================

const MAX_REQUEST_SIZE: usize = 4096;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let stdin: c_int = STDIN_FILENO;
    let stdout: c_int = STDOUT_FILENO;

    // Endless echo loop.
    loop {
        let mut buffer: [u8; MAX_REQUEST_SIZE] = [0; MAX_REQUEST_SIZE];
        let mut n: usize = 0;

        // Single echo loop where we read from STDIN until EOF.
        loop {
            let nread: c_ssize_t = match unistd::read(stdin, &mut buffer[n..]) {
                // Error encountered.
                Err(_error) => break,
                // End of file reached.
                Ok(0) => break,
                // Read some bytes.
                Ok(n) => n as c_ssize_t,
            };
            n += nread as usize;
        }

        if n > 0 {
            unistd::write(stdout, &buffer[..n])?;
        }
    }

    // The server is meant to run forever until the micro-vm is killed.
    #[allow(unreachable_code)]
    Ok(())
}
