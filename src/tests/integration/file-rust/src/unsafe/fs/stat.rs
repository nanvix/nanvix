// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::sys;
use sysapi::sys_stat::{
    self,
    file_type::S_ISREG,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can get the status of a file using `stat()`.
pub fn test() {
    let filename: &str = "README.md";

    // Get file status and assert result.
    let mut st: sys_stat::stat = sys_stat::stat::default();
    match sys::stat::stat(filename, &mut st) {
        Ok(()) => {
            // Check if the file is a regular file.
            if !S_ISREG(st.st_mode) {
                panic!("file is not a regular file");
            }
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }
}
