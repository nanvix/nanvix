// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syscall::{
    fcntl,
    safe::RegularFileOpenFlags,
    sys::stat,
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

/// Tests whether we can write and read to/from a file.
pub fn test() {
    const DATA: &[u8] = b"Hello Nanvix!";
    let filename: &str = "test-write_read.txt";
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

    // Open file for writing and assert result.
    let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::write_only().into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Write to file and assert result.
    match unistd::write(fd, DATA) {
        Ok(n) if usize::try_from(n).ok() == Some(DATA.len()) => {},
        Ok(n) => {
            panic!("expected to write {} bytes, but wrote {} bytes", DATA.len(), n);
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Close file and assert result.
    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }

    // Check if the file exists.
    let mut st: sys_stat::stat = sys_stat::stat::default();
    match stat::stat(filename, &mut st) {
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
            let expected_size: off_t = match DATA.len().try_into() {
                Ok(size) => size,
                Err(_) => {
                    panic!("failed to convert data length to off_t");
                },
            };
            if st.st_size != expected_size {
                panic!(
                    "file size does not match expected size (expected: {expected_size}, got: {})",
                    { st.st_size }
                );
            }
        },
        Err(error) => {
            panic!("file does not exist after creation: {error:?}");
        },
    }

    // Open file for reading and assert result.
    let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::read_only().into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Read from file and assert result.
    let mut expected_data: [u8; DATA.len()] = [0; DATA.len()];
    match unistd::read(fd, &mut expected_data) {
        Ok(n) if usize::try_from(n).ok() == Some(DATA.len()) => {
            if expected_data != DATA {
                panic!("expected to read {:?}, but read {:?}", DATA, expected_data);
            }
        },
        Ok(n) => {
            panic!("expected to read {} bytes, but read {n} bytes", DATA.len());
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Close file and assert result.
    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }

    // Remove the file and assert result.
    if let Err(error) = unistd::unlink(filename) {
        panic!("{error:?}");
    }
}
