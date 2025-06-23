// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syscall::{
    fcntl,
    sys,
    unistd,
};
use sysapi::{
    sys_stat::{
        self,
        file_mode::{
            S_IRUSR,
            S_IRWXG,
            S_IRWXO,
            S_IRWXU,
            S_IWUSR,
        },
        file_type::S_ISREG,
    },
    sys_types::{
        mode_t,
        off_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can allocate bytes in a file using fallocate.
pub fn test() {
    let filename: &str = "test-fallocate.txt";
    let mode: mode_t = S_IRUSR | S_IWUSR;
    let offset: off_t = 128;
    let length: off_t = 128;

    // Create a file and assert result.
    let fd: c_int = match fcntl::creat(filename, mode) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Allocate space in file using fallocate and assert result.
    if let Err(error) = fcntl::posix_fallocate(fd, offset, length) {
        panic!("{error:?}");
    }

    // Close file and assert result.
    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }

    // Check if the file exists.
    let mut st: sys_stat::stat = sys_stat::stat::default();
    match sys::stat::stat(filename, &mut st) {
        Ok(()) => {
            // Check if the file is a regular file.
            if !S_ISREG(st.st_mode) {
                panic!("file is not a regular file");
            }

            // Check if file permissions match expected permissions.
            if st.st_mode & S_IRWXU != mode
                && st.st_mode & S_IRWXG != 0
                && st.st_mode & S_IRWXO != 0
            {
                panic!(
                    "file permissions do not match expected permissions (expected: {mode:?}, got: \
                     {:?})",
                    { st.st_mode }
                );
            }

            // Check if file has expected size.
            if st.st_size != offset + length {
                panic!(
                    "file size does not match expected size (expected: {}, got: {})",
                    offset + length,
                    { st.st_size }
                );
            }
        },
        Err(error) => {
            panic!("file does not exist after creation: {error:?}");
        },
    }

    // Unlink file and assert result.
    if let Err(error) = unistd::unlink(filename) {
        panic!("{error:?}");
    }
}
