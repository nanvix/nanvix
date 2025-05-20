// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::sys::types::off_t;

//==================================================================================================
// File Offset
//==================================================================================================

///
/// # Description
///
/// A structure that represents the offset within a file.
///
pub struct FileOffset(off_t);

impl From<FileOffset> for off_t {
    fn from(offset: FileOffset) -> off_t {
        offset.0
    }
}
