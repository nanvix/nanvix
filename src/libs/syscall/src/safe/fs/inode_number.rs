// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]
#![forbid(unsafe_code)]

//==================================================================================================
// Imports
//==================================================================================================

use sysapi::sys_types::ino_t;

//==================================================================================================
// InodeNumber
//==================================================================================================

///
/// # Description
///
/// This structure represents an inode number in a filesystem.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InodeNumber(ino_t);

impl From<ino_t> for InodeNumber {
    fn from(value: ino_t) -> Self {
        InodeNumber(value)
    }
}
impl From<InodeNumber> for ino_t {
    fn from(value: InodeNumber) -> Self {
        value.0
    }
}
