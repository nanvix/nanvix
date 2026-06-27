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
    RegularFileOffset,
    RegularFileOpenFlags,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can allocate bytes in a file.
pub fn test() {
    let offset: RegularFileOffset = RegularFileOffset::from(128);
    let length: RegularFileOffset = RegularFileOffset::from(128);

    let pathname: FileSystemPath = match FileSystemPath::new("test-allocate.txt") {
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
        let mut file: RegularFile = match FileSystem::open_regular_file(
            &pathname,
            &RegularFileOpenFlags::read_write()
                .set_create(true)
                .set_truncate(true),
            Some(permissions),
        ) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        // Allocate space in file and assert result.
        if let Err(error) = file.allocate(offset, length) {
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

            // Check file system permissions.
            if attr.permissions() != permissions {
                panic!(
                    "file permissions do not match expected permissions (expected: \
                     {permissions:?}, got: {:?})",
                    attr.permissions()
                );
            }

            // Check if file size is correct.
            let expected_file_size: RegularFileOffset = match offset.checked_add(length) {
                Some(size) => size,
                None => {
                    panic!("overflow occurred while calculating file size.");
                },
            };

            if attr.size() != expected_file_size {
                panic!(
                    "file size does not match expected size (expected: {expected_file_size:?}, \
                     got: {:?})",
                    attr.size()
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
