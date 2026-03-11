// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    FileType,
    RegularFile,
    RegularFileOpenFlags,
};
use alloc::vec::Vec;

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

/// Tests whether we can write and read to/from a file.
pub fn test() {
    let filename: &str = "test-write_read.txt";
    let expected_total_len: usize = PAYLOAD_SIZES.iter().sum();

    let pathname: FileSystemPath = match FileSystemPath::new(filename) {
        Ok(pathname) => pathname,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create file and assert result.
    {
        let _file: RegularFile = match FileSystem::create_regular_file(&pathname, Some(permissions))
        {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        // File is automatically closed when it goes out of scope.
    }

    // Open file for write and write some data to it.
    {
        // Open file and assert result.
        let mut file: RegularFile = match FileSystem::open_regular_file(
            &pathname,
            &RegularFileOpenFlags::write_only(),
            None,
        ) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        for (index, &size) in PAYLOAD_SIZES.iter().enumerate() {
            let payload: Vec<u8> = make_payload(size, payload_seed(index));
            match file.write(&payload) {
                Ok(n) if n == payload.len() => {},
                Ok(n) => {
                    panic!("expected to write {} bytes, but wrote {n} bytes", payload.len());
                },
                Err(error) => {
                    panic!("{error:?}");
                },
            }
        }

        // File is automatically closed when it goes out of scope.
    }

    // Check if the file exists.
    match FileSystem::get_file_attributes(&pathname) {
        Ok(attr) => {
            // Check if the file is a regular file.
            if attr.file_type() != FileType::RegularFile {
                panic!("file is not a regular file");
            }

            // Check file has expected permissions.
            if attr.permissions() != permissions {
                panic!(
                    "file permissions do not match expected permissions (expected: \
                     {permissions:?}, got: {:?})",
                    attr.permissions()
                );
            }

            let file_size: usize = match attr.size().try_into() {
                Ok(size) => size,
                Err(error) => {
                    panic!("{error:?}");
                },
            };

            // Check if file has expected size.
            if file_size != expected_total_len {
                panic!(
                    "file size does not match expected size (expected: {}, got: {file_size})",
                    expected_total_len,
                );
            }
        },
        Err(error) => {
            panic!("file does not exist after creation: {error:?}");
        },
    }

    // Open file for read and read the data back.
    {
        // Open file and assert result.
        let file: RegularFile = match FileSystem::open_regular_file(
            &pathname,
            &RegularFileOpenFlags::read_only(),
            None,
        ) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        for (index, &size) in PAYLOAD_SIZES.iter().enumerate() {
            let expected_data: Vec<u8> = make_payload(size, payload_seed(index));
            let mut actual_data: Vec<u8> = alloc::vec![0u8; size];
            match file.read(&mut actual_data) {
                Ok(n) if n == expected_data.len() => {
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

        // File is automatically closed when it goes out of scope.
    }

    // Unlink file and assert result.
    if let Err(error) = FileSystem::remove_file(&pathname) {
        panic!("{error:?}");
    }
}
