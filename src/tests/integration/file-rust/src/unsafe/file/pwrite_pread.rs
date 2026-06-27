// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::{
    fcntl,
    unistd,
};
use sysapi::{
    fcntl::{
        file_access_mode::{
            O_RDONLY,
            O_WRONLY,
        },
        file_creation_flags::O_CREAT,
    },
    ffi::c_int,
    sys_stat::file_mode::{
        S_IRUSR,
        S_IWUSR,
    },
    sys_types::off_t,
    unistd::file_seek::SEEK_CUR,
};
use syscall::safe::RegularFileOpenFlags;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can write and read to/from a file using pwrite and pread.
pub fn test() {
    const DATA: &[u8] = b"Hello Nanvix!";
    let filename: &str = "testfile.txt";
    let offset: off_t = 128;

    // Create file and assert result.
    let fd: c_int = match fcntl::open(filename, O_CREAT | O_RDONLY, S_IRUSR | S_IWUSR) {
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
    let fd: c_int = match fcntl::open(filename, O_WRONLY, 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Write to file and assert result.
    match unistd::pwrite(fd, DATA, offset) {
        Ok(n) if usize::try_from(n).ok() == Some(DATA.len()) => {},
        Ok(n) => {
            panic!("expected to write {} bytes, but wrote {} bytes", DATA.len(), n);
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if file offset is correct.
    match unistd::lseek(fd, 0, SEEK_CUR) {
        Ok(offset) => assert_eq!(offset, 0),
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Close file and assert result.
    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
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
    match unistd::pread(fd, &mut expected_data, offset) {
        Ok(n) if usize::try_from(n).ok() == Some(DATA.len()) => {
            if expected_data != DATA {
                panic!("expected to read {:?}, but read {:?}", DATA, expected_data);
            }
        },
        Ok(n) => {
            panic!("expected to read {} bytes, but read {} bytes", DATA.len(), n);
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if file offset is correct.
    match unistd::lseek(fd, 0, SEEK_CUR) {
        Ok(offset) => assert_eq!(offset, 0),
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Close file and assert result.
    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }

    // Unlink file and assert result.
    if let Err(error) = unistd::unlink(filename) {
        panic!("{error:?}");
    }
}
