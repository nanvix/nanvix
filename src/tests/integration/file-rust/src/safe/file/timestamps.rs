// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::time::Duration;
use ::syscall::safe::{
    self,
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOpenFlags,
    time::Time,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can update file timestamps using `futimens()`.
pub fn test() {
    let filename: &str = "test-futimens.tmp";

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

        // Get current file timestamps.
        let attr = match file.attributes() {
            Ok(attr) => attr,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        let current_atime: Time = match attr.accessed() {
            Ok(time) => time,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        let current_mtime: Time = match attr.modified() {
            Ok(time) => time,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        // Set new timestamps: access time +20s, modification time +10s.
        let new_atime: Time = match current_atime.checked_add_duration(&Duration::from_secs(20)) {
            Some(time) => time,
            None => {
                panic!("overflow while computing new access time");
            },
        };

        let new_mtime: Time = match current_mtime.checked_add_duration(&Duration::from_secs(10)) {
            Some(time) => time,
            None => {
                panic!("overflow while computing new modification time");
            },
        };

        let times: [Time; 2] = [new_atime, new_mtime];

        // Update the file timestamps and check the result.
        if let Err(error) = safe::file::futimens(file.as_raw_fd(), &times) {
            panic!("{error:?}");
        }

        // Verify the updated timestamps.
        match file.attributes() {
            Ok(updated_attr) => {
                let updated_atime: Time = match updated_attr.accessed() {
                    Ok(time) => time,
                    Err(error) => {
                        panic!("{error:?}");
                    },
                };

                let updated_mtime: Time = match updated_attr.modified() {
                    Ok(time) => time,
                    Err(error) => {
                        panic!("{error:?}");
                    },
                };

                if updated_atime.seconds() != new_atime.seconds() {
                    panic!(
                        "access time mismatch (expected: {}, got: {})",
                        new_atime.seconds(),
                        updated_atime.seconds()
                    );
                }

                if updated_mtime.seconds() != new_mtime.seconds() {
                    panic!(
                        "modification time mismatch (expected: {}, got: {})",
                        new_mtime.seconds(),
                        updated_mtime.seconds()
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
