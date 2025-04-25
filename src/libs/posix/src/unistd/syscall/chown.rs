// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::types::{
    gid_t,
    uid_t,
};
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the user and group ownership of a file.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `owner`: User ID of the new owner.
/// - `group`: Group ID of the new owner.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, it returns an error.
///
pub fn chown(path: &str, owner: uid_t, group: gid_t) -> Result<(), Error> {
    ::nvx::trace!("chown(): path = {:?}, owner = {:?}, group = {:?}", path, owner, group);
    crate::unistd::fchownat(crate::fcntl::AT_FDCWD, path, owner, group, 0)
}
