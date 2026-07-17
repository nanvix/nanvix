// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Directory descriptor handle.

//==================================================================================================
// Imports
//==================================================================================================

use crate::DirEntry;
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::fat32::Fat32Error;

//==================================================================================================
// Structures
//==================================================================================================

/// Handle for an open directory.
///
/// Stores the resolved path and lazily-loaded directory entries.
/// Entries are loaded on the first `getdents()` call and returned
/// in subsequent calls via an internal cursor.
pub struct DirectoryHandle {
    /// Absolute path of the directory in the VFS.
    path: String,
    /// Cached directory entries (populated on first read).
    entries: Option<Vec<DirEntry>>,
    /// Cursor into `entries` for sequential reads.
    cursor: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl DirectoryHandle {
    /// Creates a new directory handle for the given VFS path.
    pub fn new(path: String) -> Self {
        Self {
            path,
            entries: None,
            cursor: 0,
        }
    }

    /// Returns the absolute path of this directory.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the next batch of directory entries.
    ///
    /// Lazily loads entries from the VFS on the first call and returns
    /// up to `count` entries per invocation.
    pub fn read_entries(&mut self, count: usize) -> Result<Vec<DirEntry>, Fat32Error> {
        if self.entries.is_none() {
            self.entries = Some(crate::read_dir(&self.path)?);
        }
        let all: &[DirEntry] = self.entries.as_ref().unwrap();
        let remaining: &[DirEntry] = if self.cursor < all.len() {
            &all[self.cursor..]
        } else {
            &[]
        };
        let take: usize = core::cmp::min(count, remaining.len());
        let batch: Vec<DirEntry> = remaining[..take].to_vec();
        self.cursor += take;
        Ok(batch)
    }
}
