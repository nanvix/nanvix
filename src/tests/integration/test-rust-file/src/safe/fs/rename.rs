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

/// Tests whether we can rename a file using `rename()`.
pub fn test() {
    let filename: &str = "test-rename.tmp";
    let renamed_filename: &str = "test-renamed.tmp";

    let pathname: FileSystemPath = match FileSystemPath::new(filename) {
        Ok(path) => path,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    let renamed_pathname: FileSystemPath = match FileSystemPath::new(renamed_filename) {
        Ok(path) => path,
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

    // Rename the file and assert result.
    if let Err(error) = safe::fs::rename(&pathname, &renamed_pathname) {
        panic!("{error:?}");
    }

    // Verify the renamed file exists.
    if let Err(error) = FileSystem::get_file_attributes(&renamed_pathname) {
        panic!("renamed file does not exist: {error:?}");
    }

    // Remove the renamed file.
    if let Err(error) = FileSystem::remove_file(&renamed_pathname) {
        panic!("{error:?}");
    }
}
