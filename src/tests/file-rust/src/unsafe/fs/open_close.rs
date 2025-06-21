// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syscall::{
    fcntl,
    fcntl::OpenFlags,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests wether we can open and close a file using `open()` and `close()`.
pub fn test() {
    let filename: &str = "README.md";

    // Open a file and assert result.
    let fd: c_int = match fcntl::open(filename, OpenFlags::Readonly.into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Close file and assert result.
    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }
}
