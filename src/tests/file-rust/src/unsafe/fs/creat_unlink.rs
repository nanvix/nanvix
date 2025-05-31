// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::{
    fcntl,
    fcntl::{
        S_IRUSR,
        S_IWUSR,
    },
    ffi::c_int,
    sys,
    sys::{
        stat::file_mode,
        types::mode_t,
    },
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests wether we can create a and unlink a file using `creat()` and `unlink()`.
pub fn test() {
    let filename: &str = "test-creat_unlink.txt";
    let mode: mode_t = S_IRUSR | S_IWUSR;

    // Create a file and assert result.
    let fd: c_int = match fcntl::creat(filename, mode) {
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
    let mut st: sys::stat::stat = sys::stat::stat::default();
    match sys::stat::stat(filename, &mut st) {
        Ok(()) => {
            // Check if the file is a regular file.
            if !file_mode::S_ISREG(st.st_mode) {
                panic!("file is not a regular file");
            }

            // Check if file permissions match expected permissions.
            if st.st_mode & fcntl::S_IRWXU != mode
                && st.st_mode & fcntl::S_IRWXG != 0
                && st.st_mode & fcntl::S_IRWXO != 0
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

    // Remove the file and assert result.
    if let Err(error) = unistd::unlink(filename) {
        panic!("{error:?}");
    }
}
