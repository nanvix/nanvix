// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl,
    sys::types::{
        gid_t,
        uid_t,
    },
};
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the user and group ownership of a symbolic link.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `owner`: User ID of the new owner.
/// - `group`: Group ID of the new owner.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, it returns an error is returned.
///
pub fn lchown(path: &str, owner: uid_t, group: gid_t) -> Result<(), Error> {
    ::nvx::log!("lchown(): path = {:?}, owner = {:?}, group = {:?}", path, owner, group);
    crate::fcntl::fchownat(crate::fcntl::AT_FDCWD, path, owner, group, fcntl::AT_SYMLINK_NOFOLLOW)
}
