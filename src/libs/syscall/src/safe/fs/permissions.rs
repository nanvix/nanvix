// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::sys::types::mode_t;

//==================================================================================================
// File Permissions
//==================================================================================================

///
/// # Description
///
/// A structure that represents the permissions of a file in the file system.
///
#[derive(Default, Debug)]
pub struct FileSystemPermissions(mode_t);

impl From<FileSystemPermissions> for mode_t {
    fn from(permissions: FileSystemPermissions) -> mode_t {
        permissions.0
    }
}
