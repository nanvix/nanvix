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

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can write and read to/from a file.
pub fn test() {
    const DATA: &[u8] = b"Hello Nanvix!";
    let filename: &str = "test-synchronize.txt";

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

        // Write to file and assert result.
        match file.write(DATA) {
            Ok(n) if n == DATA.len() => {
                // Successfully written.
            },
            Ok(n) => {
                panic!("expected to write {} bytes, but wrote {n} bytes", DATA.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        // File is automatically closed when it goes out of scope.
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

        // Read from file and assert result.
        let mut expected_data: [u8; DATA.len()] = [0; DATA.len()];
        match file.read(&mut expected_data) {
            Ok(n) if n == DATA.len() => {
                assert_eq!(&expected_data[..DATA.len()], DATA);
            },
            Ok(n) => {
                panic!("expected to read {} bytes, but read {} bytes", DATA.len(), n);
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        // Synchronize changes to the file and assert result.
        if let Err(error) = file.synchronize() {
            panic!("{error:?}");
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
            if file_size != DATA.len() {
                panic!(
                    "file size does not match expected size (expected: {}, got: {file_size})",
                    DATA.len(),
                );
            }
        },
        Err(error) => {
            panic!("file does not exist after creation: {error:?}");
        },
    }

    // Unlink file and assert result.
    if let Err(error) = FileSystem::remove_file(&pathname) {
        panic!("{error:?}");
    }
}
