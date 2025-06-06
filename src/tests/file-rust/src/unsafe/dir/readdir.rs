// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystemPath,
    dir::{
        RawDirectory,
        closedir,
        opendir,
        readdir,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can read entries from a directory using `readdir()`.
pub fn test() {
    let dirname: FileSystemPath = match FileSystemPath::new(".") {
        Ok(path) => path,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Open a directory and assert result.
    let mut dir: RawDirectory = match opendir(&dirname) {
        Ok(dir) => dir,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Traverse all directory entries looking for `README.md` file.
    let mut count: usize = 0;
    let mut found: bool = false;
    while let Ok(Some(raw_entry)) = readdir(&mut dir) {
        match raw_entry.file_name() {
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

    // Close directory and assert result.
    if let Err(error) = closedir(&dir) {
        panic!("{error:?}");
    }
}
