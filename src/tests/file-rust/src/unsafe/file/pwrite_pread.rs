// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use alloc::vec::Vec;
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
use syscall::{
    fcntl,
    safe::RegularFileOpenFlags,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

const PAYLOAD_SIZES: &[usize] = &[32, 4096, 4097, 8192, 16384, 32768, 65536];
const INITIAL_OFFSET: off_t = 128;
const OFFSET_GAP: off_t = 97;
const SHORT_READ_DELTA: usize = 37;
const UNREAD_SENTINEL: u8 = 0xcc;

fn payload_seed(index: usize) -> u8 {
    match u8::try_from(index & 0xff) {
        Ok(seed) => seed,
        Err(error) => {
            panic!("{error:?}");
        },
    }
}

fn make_payload(size: usize, seed: u8) -> Vec<u8> {
    let mut payload: Vec<u8> = alloc::vec![0u8; size];
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte = seed.wrapping_add(payload_seed(index));
    }
    payload
}

fn short_read_len(requested_size: usize) -> usize {
    if requested_size <= 64 {
        core::cmp::max(1, requested_size / 2)
    } else {
        requested_size - SHORT_READ_DELTA
    }
}

fn test_short_pread() {
    let filename: &str = "test-short-pread.txt";

    for (index, &requested_size) in PAYLOAD_SIZES.iter().enumerate() {
        let payload_size: usize = short_read_len(requested_size);
        let payload: Vec<u8> = make_payload(payload_size, payload_seed(index));

        let fd: c_int = match fcntl::creat(filename, S_IRUSR | S_IWUSR) {
            Ok(fd) => fd,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        if let Err(error) = unistd::close(fd) {
            panic!("{error:?}");
        }

        let fd: c_int = match fcntl::open(filename, O_WRONLY, 0) {
            Ok(fd) => fd,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        match unistd::pwrite(fd, &payload, 0) {
            Ok(n) if usize::try_from(n).ok() == Some(payload.len()) => {},
            Ok(n) => {
                panic!("expected to write {} bytes, but wrote {n} bytes", payload.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        if let Err(error) = unistd::close(fd) {
            panic!("{error:?}");
        }

        let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::read_only().into(), 0) {
            Ok(fd) => fd,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        let mut actual_data: Vec<u8> = alloc::vec![UNREAD_SENTINEL; requested_size];
        match unistd::pread(fd, &mut actual_data, 0) {
            Ok(n) if usize::try_from(n).ok() == Some(payload.len()) => {
                if actual_data[..payload.len()] != payload[..] {
                    panic!(
                        "short pread payload mismatch for requested size {requested_size} \
                         (expected prefix byte {}, got {})",
                        payload[0], actual_data[0],
                    );
                }
                if actual_data[payload.len()..]
                    .iter()
                    .any(|byte| *byte != UNREAD_SENTINEL)
                {
                    panic!(
                        "short pread clobbered unread suffix for requested size {requested_size}"
                    );
                }
            },
            Ok(n) => {
                panic!("expected to read {} bytes, but read {n} bytes", payload.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        match unistd::lseek(fd, 0, SEEK_CUR) {
            Ok(offset) => assert_eq!(offset, 0),
            Err(error) => {
                panic!("{error:?}");
            },
        };

        if let Err(error) = unistd::close(fd) {
            panic!("{error:?}");
        }
    }

    if let Err(error) = unistd::unlink(filename) {
        panic!("{error:?}");
    }
}

/// Tests whether we can write and read to/from a file using pwrite and pread.
pub fn test() {
    let filename: &str = "testfile.txt";
    let mut offsets: Vec<off_t> = Vec::with_capacity(PAYLOAD_SIZES.len());
    let mut next_offset: off_t = INITIAL_OFFSET;

    for &size in PAYLOAD_SIZES {
        offsets.push(next_offset);
        let size_offset: off_t = match off_t::try_from(size) {
            Ok(size) => size,
            Err(error) => {
                panic!("{error:?}");
            },
        };
        next_offset += size_offset + OFFSET_GAP;
    }

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

    for (index, (&size, &offset)) in PAYLOAD_SIZES.iter().zip(offsets.iter()).enumerate() {
        let payload: Vec<u8> = make_payload(size, payload_seed(index));
        match unistd::pwrite(fd, &payload, offset) {
            Ok(n) if usize::try_from(n).ok() == Some(payload.len()) => {},
            Ok(n) => {
                panic!("expected to write {} bytes, but wrote {} bytes", payload.len(), n);
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }
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

    for (index, (&size, &offset)) in PAYLOAD_SIZES.iter().zip(offsets.iter()).enumerate() {
        let expected_data: Vec<u8> = make_payload(size, payload_seed(index));
        let mut actual_data: Vec<u8> = alloc::vec![0u8; size];
        match unistd::pread(fd, &mut actual_data, offset) {
            Ok(n) if usize::try_from(n).ok() == Some(expected_data.len()) => {
                if actual_data != expected_data {
                    panic!(
                        "payload mismatch for size {size} at offset {offset} (expected prefix \
                         byte {}, got {})",
                        expected_data[0], actual_data[0],
                    );
                }
            },
            Ok(n) => {
                panic!("expected to read {} bytes, but read {} bytes", expected_data.len(), n);
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }
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

    test_short_pread();
}
