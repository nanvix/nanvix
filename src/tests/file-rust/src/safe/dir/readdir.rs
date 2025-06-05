// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    dir::Directory,
};
//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can read entries from a directory.
pub fn test() {
    let dirname: FileSystemPath = match FileSystemPath::new(".") {
        Ok(path) => path,
        Err(error) => {
            panic!("Failed to create FileSystemPath: {error:?}");
        },
    };

    // Open a directory and assert result.
    let mut dir: Directory = match FileSystem::open_directory(&dirname) {
        Ok(dir) => dir,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Traverse all directory entries looking for `README.md` file.
    let mut count: usize = 0;
    let mut found: bool = false;
    while let Some(Ok(entry)) = dir.next() {
        match entry.file_name() {
            Ok(name) => {
                if name == "README.md" {
                    found = true;
                    // Continue traversing all other entries.
                }
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }
        count += 1;
    }

    // Assert that we found the `README.md` file.
    if !found {
        panic!("`README.md` file not found in directory `{dirname:?}`");
    }

    // Assert if number of entries matched what we expected.
    if count < 1 {
        panic!("expected at three entries in directory `{dirname:?}`, found {count}");
    }

    // Directory is automatically closed when it goes out of scope.
}
