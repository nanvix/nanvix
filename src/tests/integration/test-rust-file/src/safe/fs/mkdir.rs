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
    FileType,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can create and remove a directory using `mkdir()`.
pub fn test() {
    let dirname: &str = "test-mkdir-dir";

    let dirpath: FileSystemPath = match FileSystemPath::new(dirname) {
        Ok(path) => path,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true)
        .user_execute(true);

    // Create a test directory.
    if let Err(error) = safe::fs::mkdir(&dirpath, permissions) {
        panic!("{error:?}");
    }

    // Check if the directory exists with expected attributes.
    match FileSystem::get_file_attributes(&dirpath) {
        Ok(attr) => {
            if attr.file_type() != FileType::Directory {
                panic!("expected directory, got {:?}", attr.file_type());
            }
            if !attr.permissions().user_can_read() {
                panic!("expected user read permission");
            }
            if !attr.permissions().user_can_write() {
                panic!("expected user write permission");
            }
            if !attr.permissions().user_can_execute() {
                panic!("expected user execute permission");
            }
        },
        Err(error) => {
            panic!("directory does not exist after creation: {error:?}");
        },
    }

    // Remove the test directory using unlinkat with AT_REMOVEDIR.
    if let Err(error) = ::syscall::fcntl::syscall::unlinkat(
        sysapi::fcntl::atflags::AT_FDCWD,
        dirname,
        sysapi::fcntl::atflags::AT_REMOVEDIR,
    ) {
        panic!("{error:?}");
    }
}
