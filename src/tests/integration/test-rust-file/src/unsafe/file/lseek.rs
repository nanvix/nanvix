// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syscall::{
    fcntl,
    unistd,
};
use sysapi::{
    sys_stat::file_mode::{
        S_IRUSR,
        S_IWUSR,
    },
    sys_types::{
        c_size_t,
        mode_t,
        off_t,
    },
    unistd::file_seek::{
        SEEK_END,
        SEEK_SET,
    },
};
use syscall::safe::RegularFileOpenFlags;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can manipulate the seek position of a file.
pub fn test() {
    const DATA: &[u8] = b"Hello Nanvix!";
    let filename: &str = "test-lseek.txt";

    // Create a file and assert result.
    let mode: mode_t = S_IRUSR | S_IWUSR;
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

    // Open file for reading and writing and assert result.
    let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::read_write().into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Write tot he beginning of the file and assert result.
    match unistd::write(fd, DATA) {
        Ok(n) if usize::try_from(n).ok() == Some(DATA.len()) => {},
        Ok(n) => {
            panic!("expected to write {} bytes, but wrote {n} bytes", DATA.len());
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Seek to the beginning of the file and assert result.
    if let Err(error) = unistd::lseek(fd, 0, SEEK_SET) {
        panic!("{error:?}");
    }

    // Read from the file and assert result.
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

    // Open file for reading and writing again and assert result.
    let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::read_write().into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Seek to the end of the file and assert result.
    if let Err(error) = unistd::lseek(fd, 0, SEEK_END) {
        panic!("{error:?}");
    }

    // Write to the end of the file and assert result.
    match unistd::write(fd, DATA) {
        Ok(n) if usize::try_from(n).ok() == Some(DATA.len()) => {},
        Ok(n) => {
            panic!("expected to write {} bytes, but wrote {n} bytes", DATA.len());
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Rewind from the end of the file.
    let offset: i64 = match off_t::try_from(DATA.len()) {
        Ok(offset) => -offset,
        Err(_) => {
            panic!("failed to convert length to i32");
        },
    };
    if let Err(error) = unistd::lseek(fd, offset, SEEK_END) {
        panic!("{error:?}");
    }

    // Read data back and assert result.
    let expected_size: c_size_t = match DATA.len().try_into() {
        Ok(size) => size,
        Err(_) => {
            panic!("failed to convert data length to size_t");
        },
    };
    let mut expected_data: [u8; DATA.len()] = [0; DATA.len()];
    match unistd::read(fd, &mut expected_data) {
        Ok(n) if n == expected_size => {
            assert_eq!(&expected_data[..DATA.len()], DATA);
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

    // Unlink the file and assert result.
    if let Err(error) = unistd::unlink(filename) {
        panic!("{error:?}");
    }
}
