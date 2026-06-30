// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::alloc::{
    vec,
    vec::Vec,
};
use ::sys::error::Error;
use ::sysapi::{
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

// Keep this independent from benchmark CLI options: this app is reused by multiple benchmark modes.
const MAX_REQUEST_SIZE: usize = 64 * 1024;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let stdin: i32 = STDIN_FILENO;
    let stdout: i32 = STDOUT_FILENO;
    let mut buffer: Vec<u8> = vec![0; MAX_REQUEST_SIZE];

    loop {
        let nread: c_ssize_t = match unistd::read(stdin, &mut buffer) {
            // Error encountered.
            Err(_error) => break,
            // End of file reached.
            Ok(0) => break,
            // Read some bytes.
            Ok(n) => n as c_ssize_t,
        };

        unistd::write(stdout, &buffer[..nread as usize])?;
    }

    Ok(())
}
