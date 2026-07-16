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
    ffi::{
        c_int,
        c_void,
    },
    sys_ioctl::{
        TCGETS,
        TCSETS,
    },
    sys_types::c_ssize_t,
    termios::{
        Termios,
        ECHO,
        ECHOE,
        ECHOK,
        ICANON,
        ICRNL,
        IEXTEN,
        ISIG,
        IXON,
        ONLCR,
        OPOST,
        VMIN,
        VTIME,
    },
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

/// Configures the shared console as a byte-oriented stream for the echo workload.
fn configure_raw_terminal(fd: c_int) -> Result<(), Error> {
    let mut termios: Termios = Termios::console_default();

    // SAFETY: the argument points to a valid, writable `Termios` for the duration of the call.
    if unsafe { ::syscall::sys::ioctl::ioctl(fd, TCGETS, (&raw mut termios).cast::<c_void>()) }
        .is_err()
    {
        // The direct VMM benchmark exposes a byte stream without terminal ioctl support.
        return Ok(());
    }

    termios.c_iflag &= !(ICRNL | IXON);
    termios.c_oflag &= !(OPOST | ONLCR);
    termios.c_lflag &= !(ISIG | ICANON | ECHO | ECHOE | ECHOK | IEXTEN);
    termios.c_cc[VMIN] = 1;
    termios.c_cc[VTIME] = 0;

    // SAFETY: the argument points to a valid, readable `Termios` for the duration of the call.
    unsafe { ::syscall::sys::ioctl::ioctl(fd, TCSETS, (&raw mut termios).cast::<c_void>()) }?;

    Ok(())
}

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let stdin: c_int = STDIN_FILENO;
    let stdout: c_int = STDOUT_FILENO;
    let mut buffer: Vec<u8> = vec![0; MAX_REQUEST_SIZE];

    configure_raw_terminal(stdin)?;

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
