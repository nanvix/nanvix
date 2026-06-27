// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    self,
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can change access permissions of a file using `chmod()`.
pub fn test() {
    let filename: &str = "test-chmod.tmp";

    let pathname: FileSystemPath = match FileSystemPath::new(filename) {
        Ok(pathname) => pathname,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create a test file.
    {
        let _file: RegularFile = match FileSystem::create_regular_file(&pathname, Some(permissions))
        {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };
    }

    // Save current access permissions.
    let original_permissions: FileSystemPermissions =
        match FileSystem::get_file_attributes(&pathname) {
            Ok(attr) => attr.permissions(),
            Err(error) => {
                panic!("{error:?}");
            },
        };

    // Change access permissions: remove group and other read.
    let new_permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true)
        .group_read(false)
        .others_read(false);

    if let Err(error) = safe::fs::chmod(&pathname, new_permissions) {
        panic!("{error:?}");
    }

    // Verify the permissions changed.
    match FileSystem::get_file_attributes(&pathname) {
        Ok(attr) => {
            if attr.permissions().group_can_read() {
                panic!("expected group read permission to be cleared");
            }
            if attr.permissions().others_can_read() {
                panic!("expected others read permission to be cleared");
            }
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Restore original access permissions.
    if let Err(error) = safe::fs::chmod(&pathname, original_permissions) {
        panic!("{error:?}");
    }

    // Verify the permissions were restored.
    match FileSystem::get_file_attributes(&pathname) {
        Ok(attr) => {
            if attr.permissions() != original_permissions {
                panic!(
                    "permissions were not restored (expected: {original_permissions:?}, got: {:?})",
                    attr.permissions()
                );
            }
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Remove the test file.
    if let Err(error) = FileSystem::remove_file(&pathname) {
        panic!("{error:?}");
    }
}
