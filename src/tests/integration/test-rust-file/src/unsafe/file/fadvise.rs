// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    fcntl::file_advice::POSIX_FADV_SEQUENTIAL,
    ffi::c_int,
};
use ::syscall::{
    fcntl,
    safe::RegularFileOpenFlags,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can use posix_fadvise on a file.
pub fn test() {
    let filename: &str = "README.md";

    // Open file and assert result.
    let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::read_only().into(), 0) {
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
