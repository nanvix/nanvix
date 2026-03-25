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
    RegularFileOpenFlags,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can change access permissions of a file descriptor using `fchmod()`.
pub fn test() {
    let filename: &str = "test-fchmod.tmp";

    let pathname: FileSystemPath = match FileSystemPath::new(filename) {
        Ok(pathname) => pathname,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create and open a test file.
    {
        let file: RegularFile = match FileSystem::open_regular_file(
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

        // Save current access permissions.
        let original_permissions: FileSystemPermissions = match file.attributes() {
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

        if let Err(error) = safe::file::fchmod(file.as_raw_fd(), new_permissions) {
            panic!("{error:?}");
        }

        // Verify the permissions changed.
        match file.attributes() {
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
        if let Err(error) = safe::file::fchmod(file.as_raw_fd(), original_permissions) {
            panic!("{error:?}");
        }

        // Verify the permissions were restored.
        match file.attributes() {
            Ok(attr) => {
                if attr.permissions() != original_permissions {
                    panic!(
                        "permissions were not restored (expected: {original_permissions:?}, got: \
                         {:?})",
                        attr.permissions()
                    );
                }
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        // File is automatically closed when it goes out of scope.
    }

    // Remove the test file.
    if let Err(error) = FileSystem::remove_file(&pathname) {
        panic!("{error:?}");
    }
}
