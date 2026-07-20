// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Mounted FAT filesystem.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;
use ::fat32::{
    Fat,
    Fat32Error,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A single mount point in the VFS.
pub struct Mount {
    /// Absolute path where this mount is rooted.
    path: String,
    /// FAT filesystem mounted at this path.
    fat: Fat,
    /// Whether the mount rejects mutating operations.
    readonly: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Mount {
    /// Creates a mount point.
    ///
    /// # Errors
    ///
    /// Returns [`Fat32Error::InvalidPath`] when `path` is not absolute.
    pub fn new(path: String, fat: Fat, readonly: bool) -> Result<Self, Fat32Error> {
        if !path.starts_with('/') {
            return Err(Fat32Error::InvalidPath);
        }
        Ok(Self {
            path,
            fat,
            readonly,
        })
    }

    /// Returns the mount path.
    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the mounted FAT filesystem.
    #[inline]
    pub fn fat(&self) -> &Fat {
        &self.fat
    }

    /// Returns the mounted FAT filesystem mutably.
    #[inline]
    pub fn fat_mut(&mut self) -> &mut Fat {
        &mut self.fat
    }

    /// Returns whether this mount is read-only.
    #[inline]
    pub fn readonly(&self) -> bool {
        self.readonly
    }

    /// Returns the path relative to this mount when `path` belongs to it.
    pub fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        if self.path == "/" {
            Some(path.strip_prefix('/').unwrap_or(path))
        } else if path == self.path {
            Some("")
        } else if path.starts_with(&self.path) {
            let rest: &str = &path[self.path.len()..];
            rest.strip_prefix('/')
        } else {
            None
        }
    }
}
