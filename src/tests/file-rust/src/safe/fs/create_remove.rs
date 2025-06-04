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
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can create and unlink a file.
pub fn test() {
    let pathname: FileSystemPath = match FileSystemPath::new("test-open_unlink.txt") {
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
