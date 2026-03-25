// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    self,
    FileSystemPath,
    FileType,
    RegularFileOffset,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can get detailed file status information using `stat()`.
pub fn test() {
    let filename: &str = "README.md";

    let pathname: FileSystemPath = match FileSystemPath::new(filename) {
        Ok(path) => path,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Get file status information and assert results.
    match safe::fs::stat(&pathname) {
        Ok(attr) => {
            // Check file type.
            if attr.file_type() != FileType::RegularFile {
                panic!("expected regular file, got {:?}", attr.file_type());
            }

            // Check file size is positive.
            if attr.size() <= RegularFileOffset::from(0) {
                panic!("expected positive file size");
            }

            // Check access time is valid.
            match attr.accessed() {
                Ok(time) => {
                    if time.seconds() == 0 {
                        panic!("expected non-zero access time");
                    }
                },
                Err(error) => {
                    panic!("failed to get access time: {error:?}");
                },
            }

            // Check modification time is valid.
            match attr.modified() {
                Ok(time) => {
                    if time.seconds() == 0 {
                        panic!("expected non-zero modification time");
                    }
                },
                Err(error) => {
                    panic!("failed to get modification time: {error:?}");
                },
            }
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }
}
