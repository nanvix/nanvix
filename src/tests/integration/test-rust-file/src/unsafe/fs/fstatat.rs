// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    sys_stat::{
        self,
        file_type::S_ISREG,
    },
};
use ::syscall::sys;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can get the status of a file using `fstatat().
pub fn test() {
    let filename: &str = "README.md";

    let mut st: sys_stat::stat = sys_stat::stat::default();
    match sys::stat::fstatat(AT_FDCWD, filename, &mut st, 0) {
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
