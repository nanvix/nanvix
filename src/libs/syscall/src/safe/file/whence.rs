// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::{
    ffi::c_int,
    unistd,
};

//==================================================================================================
// File Whence
//==================================================================================================

///
/// # Description
///
/// A type representing the whence of a file seek operation.
///
#[repr(i32)]
pub enum FileSeekWhence {
    /// The offset is set to `offset`.
    Set = unistd::SEEK_SET,
    /// The offset is set to its current location plus `offset`.
    Cur = unistd::SEEK_CUR,
    /// The offset is set to the end of the file plus `offset`.
    End = unistd::SEEK_END,
}

impl From<FileSeekWhence> for c_int {
    fn from(whence: FileSeekWhence) -> c_int {
        whence as c_int
    }
}
