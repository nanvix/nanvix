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
use alloc::vec::Vec;
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

const PAYLOAD_SIZES: &[usize] = &[32, 4096, 4097, 8192, 16384, 32768, 65536];

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
}
