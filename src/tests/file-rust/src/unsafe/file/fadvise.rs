// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::{
    fcntl,
    fcntl::{
        OpenFlags,
        POSIX_FADV_SEQUENTIAL,
    },
    ffi::c_int,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests wether we can use posix_fadvise on a file.
pub fn test() {
    let filename: &str = "README.md";

    // Open file and assert result.
    let fd: c_int = match fcntl::open(filename, OpenFlags::O_RDONLY.into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Give advice about file access.
    if let Err(error) = fcntl::posix_fadvise(fd, 0, 0, POSIX_FADV_SEQUENTIAL) {
        panic!("{error:?}");
    }

    // Close file and assert result.
    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }
}
