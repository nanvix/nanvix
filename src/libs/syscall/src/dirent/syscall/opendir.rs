// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::DirectoryStream,
    fcntl,
    safe::OpenFlags,
};
use ::alloc::boxed::Box;
use ::sys::error::Error;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Opens a directory stream.
pub fn opendir(dirname: &str) -> Result<Box<DirectoryStream>, Error> {
    let flags: c_int = OpenFlags::read_only().set_directory(true).into();
    let fd: c_int = fcntl::open(dirname, flags, 0)?;
    let dir: DirectoryStream = DirectoryStream::new(fd);
    Ok(Box::new(dir))
}
