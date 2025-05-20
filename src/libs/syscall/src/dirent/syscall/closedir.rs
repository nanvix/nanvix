// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::DirectoryStream,
    unistd,
};
use ::alloc::boxed::Box;
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Closes a directory stream.
pub fn closedir(dirp: &mut Box<DirectoryStream>) -> Result<(), Error> {
    // Drain all entries in the directory stream.
    while let Some(_posix_dirent) = dirp.pop() {}

    unistd::close(dirp.fd())
}
