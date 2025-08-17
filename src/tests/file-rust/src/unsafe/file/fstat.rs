// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sys_stat::{
        self,
        file_type::S_ISREG,
    },
};
use ::syscall::{
    fcntl,
    safe::RegularFileOpenFlags,
    sys,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can get the status of a file using fstat.
pub fn test() {
    let filename: &str = "README.md";

    // Open a file and assert result.
    let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::read_only().into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Get file status and assert result.
    let mut st: sys_stat::stat = sys_stat::stat::default();
    match sys::stat::fstat(fd, &mut st) {
        Ok(()) => {
            // Check if the file is a regular file.
            if !S_ISREG(st.st_mode) {
                panic!("file is not a regular file");
            }
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
