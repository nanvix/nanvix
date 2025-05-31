// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::{
    fcntl::AT_FDCWD,
    sys,
    sys::stat::file_mode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests wether we can get the status of a file using `fstatat().
pub fn test() {
    let filename: &str = "README.md";

    let mut st: sys::stat::stat = sys::stat::stat::default();
    match sys::stat::fstatat(AT_FDCWD, filename, &mut st, 0) {
        Ok(()) => {
            // Check if the file is a regular file.
            if !file_mode::S_ISREG(st.st_mode) {
                panic!("file is not a regular file");
            }
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }
}
