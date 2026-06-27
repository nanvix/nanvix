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
    fcntl::{
        file_access_mode::O_RDWR,
        file_creation_flags::{
            O_CREAT,
            O_TRUNC,
        },
    },
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
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can create and unlink a file using `open()` and `unlink()`.
pub fn test() {
    let filename: &str = "test-open_unlink.txt";
    let mode: mode_t = S_IRUSR | S_IWUSR;

    // Create file and assert result.
    let fd: c_int = match fcntl::open(filename, O_CREAT | O_RDWR | O_TRUNC, mode) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

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
