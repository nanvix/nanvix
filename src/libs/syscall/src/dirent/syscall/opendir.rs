// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::DirectoryStream,
    fcntl::{
        self,
        OpenFlags,
    },
};
use ::alloc::boxed::Box;
use ::sys::error::Error;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Opens a directory stream.
pub fn opendir(dirname: &str) -> Result<Box<DirectoryStream>, Error> {
    let fd: c_int = fcntl::open(dirname, OpenFlags::Readonly | OpenFlags::Directory, 0)?;
    let dir: DirectoryStream = DirectoryStream::new(fd);
    Ok(Box::new(dir))
}
