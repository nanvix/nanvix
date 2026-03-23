// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use alloc::vec::Vec;
use sysapi::{
    ffi::c_int,
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
use syscall::{
    fcntl,
    safe::RegularFileOpenFlags,
    sys::stat,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

const PAYLOAD_SIZES: &[usize] = &[32, 4096, 4097, 8192, 16384, 32768, 65536];
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

fn test_short_reads() {
    let filename: &str = "test-short-write_read.txt";
    let mode: mode_t = S_IRUSR | S_IWUSR;

    for (index, &requested_size) in PAYLOAD_SIZES.iter().enumerate() {
        let payload_size: usize = short_read_len(requested_size);
        let payload: Vec<u8> = make_payload(payload_size, payload_seed(index));

        let fd: c_int = match fcntl::creat(filename, mode) {
            Ok(fd) => fd,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        if let Err(error) = unistd::close(fd) {
            panic!("{error:?}");
        }

        let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::write_only().into(), 0) {
            Ok(fd) => fd,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        match unistd::write(fd, &payload) {
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
        match unistd::read(fd, &mut actual_data) {
            Ok(n) if usize::try_from(n).ok() == Some(payload.len()) => {
                if actual_data[..payload.len()] != payload[..] {
                    panic!(
                        "short read payload mismatch for requested size {requested_size} \
                         (expected prefix byte {}, got {})",
                        payload[0], actual_data[0],
                    );
                }
                if actual_data[payload.len()..]
                    .iter()
                    .any(|byte| *byte != UNREAD_SENTINEL)
                {
                    panic!(
                        "short read clobbered unread suffix for requested size {requested_size}"
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

        if let Err(error) = unistd::close(fd) {
            panic!("{error:?}");
        }
    }

    if let Err(error) = unistd::unlink(filename) {
        panic!("{error:?}");
    }
}

/// Tests whether we can write and read multi-page payloads to/from a file.
pub fn test() {
    let filename: &str = "test-write_read.txt";
    let mode: mode_t = S_IRUSR | S_IWUSR;
    let expected_total_len: usize = PAYLOAD_SIZES.iter().sum();

    let fd: c_int = match fcntl::creat(filename, mode) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }

    let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::write_only().into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    for (index, &size) in PAYLOAD_SIZES.iter().enumerate() {
        let payload: Vec<u8> = make_payload(size, payload_seed(index));
        match unistd::write(fd, &payload) {
            Ok(n) if usize::try_from(n).ok() == Some(payload.len()) => {},
            Ok(n) => {
                panic!("expected to write {} bytes, but wrote {} bytes", payload.len(), n);
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }
    }

    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }

    let mut st: sys_stat::stat = sys_stat::stat::default();
    match stat::stat(filename, &mut st) {
        Ok(()) => {
            if !S_ISREG(st.st_mode) {
                panic!("file is not a regular file");
            }

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

            let expected_size: off_t = match expected_total_len.try_into() {
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

    let fd: c_int = match fcntl::open(filename, RegularFileOpenFlags::read_only().into(), 0) {
        Ok(fd) => fd,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    for (index, &size) in PAYLOAD_SIZES.iter().enumerate() {
        let expected_data: Vec<u8> = make_payload(size, payload_seed(index));
        let mut actual_data: Vec<u8> = alloc::vec![0u8; size];
        match unistd::read(fd, &mut actual_data) {
            Ok(n) if usize::try_from(n).ok() == Some(expected_data.len()) => {
                if actual_data != expected_data {
                    panic!(
                        "payload mismatch for size {size} (expected prefix byte {}, got {})",
                        expected_data[0], actual_data[0],
                    );
                }
            },
            Ok(n) => {
                panic!("expected to read {} bytes, but read {n} bytes", expected_data.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }
    }

    if let Err(error) = unistd::close(fd) {
        panic!("{error:?}");
    }

    if let Err(error) = unistd::unlink(filename) {
        panic!("{error:?}");
    }

    test_short_reads();
}
