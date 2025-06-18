// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::dirent::{
    posix_dent,
    posix_getdents,
    DirectoryStream,
};
use ::alloc::{
    boxed::Box,
    vec::Vec,
};
use ::sys::error::Error;
use sysapi::dirent::dirent;

//==================================================================================================
// Constants
//==================================================================================================

/// Minimum number of entries to get when refilling buffers.
const REFILL_COUNT: usize = 1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn readdir(dir: &mut Box<DirectoryStream>) -> Result<Option<dirent>, Error> {
    ::syslog::trace!("readdir(): dir.fd={:?}", dir.fd());

    if let Some(posix_dirent) = dir.pop() {
        let dirent: dirent = posix_dirent.into();
        return Ok(Some(dirent));
    }

    // Refill buffer.
    let mut entries: Vec<posix_dent> = match posix_getdents(dir.fd, REFILL_COUNT) {
        Ok(entries) => entries,
        Err(error) => {
            ::syslog::warn!("readdir(): {error:?} (dir.fd={:?})", dir.fd());
            return Err(error);
        },
    };

    // Get next entry.
    let dirent: dirent = match entries.pop() {
        Some(posix_dirent) => posix_dirent.into(),
        None => return Ok(None),
    };

    // Push remaining entries to the directory stream.
    while let Some(entry) = entries.pop() {
        dir.push(entry);
    }

    Ok(Some(dirent))
}
