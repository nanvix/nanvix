// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::DirectoryStream,
    fcntl::{
        self,
        O_DIRECTORY,
        O_RDONLY,
    },
    ffi::c_int,
};
use ::alloc::boxed::Box;
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Opens a directory stream.
pub fn opendir(dirname: &str) -> Result<Box<DirectoryStream>, Error> {
    let fd: c_int = fcntl::open(dirname, O_RDONLY | O_DIRECTORY, 0)?;
    let dir: DirectoryStream = DirectoryStream::new(fd);
    Ok(Box::new(dir))
}
