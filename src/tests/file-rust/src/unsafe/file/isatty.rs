// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    unistd::{
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::syscall::{
    fcntl,
    unistd,
};
use syscall::safe::RegularFileOpenFlags;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can test if a file descriptor is a terminal using `isatty`.
pub fn test() {
    // Check if STDIN is a terminal.
    match unistd::isatty(STDIN_FILENO) {
        Ok(true) => {},
        Ok(false) => {
            panic!("expected STDIN to be a terminal, but it is not");
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if STDOUT is a terminal.
    match unistd::isatty(STDOUT_FILENO) {
        Ok(true) => {},
        Ok(false) => {
            panic!("expected STDOUT to be a terminal, but it is not");
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if STDERR is a terminal.
    match unistd::isatty(STDERR_FILENO) {
        Ok(true) => {},
        Ok(false) => {
            panic!("expected STDERR to be a terminal, but it is not");
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if a regular file is a terminal.
    {
        let filename: &str = "README.md";

        // Open file and assert result.
        let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::read_only().into(), 0) {
            Ok(fd) => fd,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        // Check if the file descriptor is a terminal.
        match unistd::isatty(fd) {
            Ok(false) => {},
            Ok(true) => {
                panic!("expected file descriptor to not be a terminal, but it is");
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        // Close file and assert result.
        if let Err(error) = unistd::close(fd) {
            panic!("{error:?}");
        }
    }
}
