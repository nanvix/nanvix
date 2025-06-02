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
pub enum RegularFileSeekWhence {
    /// The offset is set to the beginning of the file plus `offset`.
    Start = unistd::SEEK_SET,
    /// The offset is set to its current location plus `offset`.
    Current = unistd::SEEK_CUR,
    /// The offset is set to the end of the file plus `offset`.
    End = unistd::SEEK_END,
}

impl From<RegularFileSeekWhence> for c_int {
    fn from(whence: RegularFileSeekWhence) -> c_int {
        whence as c_int
    }
}
