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
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Closes a directory stream.
pub fn closedir(mut dir: Box<DirectoryStream>) -> Result<(), Error> {
    // Drain all entries in the directory stream.
    while let Some(_) = dir.pop() {}

    unistd::close(dir.fd())
}
