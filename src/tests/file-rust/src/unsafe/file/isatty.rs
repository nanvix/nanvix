// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::{
    fcntl,
    fcntl::OpenFlags,
    ffi::c_int,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests wether we can test if a file descriptor is a terminal using `isatty`.
pub fn test() {
    // Check if STDIN is a terminal.
    match unistd::isatty(unistd::STDIN_FILENO) {
        Ok(true) => {},
        Ok(false) => {
            panic!("expected STDIN to be a terminal, but it is not");
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if STDOUT is a terminal.
    match unistd::isatty(unistd::STDOUT_FILENO) {
        Ok(true) => {},
        Ok(false) => {
            panic!("expected STDOUT to be a terminal, but it is not");
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if STDERR is a terminal.
    match unistd::isatty(unistd::STDERR_FILENO) {
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
        let fd: c_int = match fcntl::open(filename, OpenFlags::O_RDONLY.into(), 0) {
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
