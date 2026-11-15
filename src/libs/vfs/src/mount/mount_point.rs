// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Mounted FAT filesystem.

//==================================================================================================
// Imports
//==================================================================================================

use super::metadata_cache::MetadataCache;
use crate::filesystem::Stat;
use ::alloc::string::String;
use ::fat32::{
    Fat,
    Fat32Error,
    FatResolvedPath,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A single mount point in the VFS.
///
/// # Metadata Cache
///
/// A mount caches the answers to the metadata queries that it serves, see [`Mount::stat()`]. The
/// cache is validated against the modification epoch of the mounted filesystem, so it stays correct
/// whether the filesystem is modified through the [`Fat`] that [`Mount::fat()`] returns, or through
/// the files that are opened from it. The cache lives and dies with the filesystem, because a
/// mount never gives its [`Fat`] out by value or by mutable reference.
pub struct Mount {
    /// Absolute path where this mount is rooted.
    path: String,
    /// FAT filesystem mounted at this path.
    fat: Fat,
    /// Whether the mount rejects mutating operations.
    readonly: bool,
    /// Cache of the metadata queries answered by `fat`.
    cache: MetadataCache,
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
            cache: MetadataCache::new(),
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

    /// Returns whether this mount is read-only.
    #[inline]
    pub fn readonly(&self) -> bool {
        self.readonly
    }

    /// Gets file or directory metadata, answering from the metadata cache when it can.
    ///
    /// The answer is always the one that [`Fat::stat()`] would give now, because the cache is
    /// validated against the modification epoch of the filesystem.
    ///
    /// # Parameters
    ///
    /// - `path`: Checked path relative to this mount.
    ///
    /// # Returns
    ///
    /// The metadata of the entry.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if the path doesn't exist.
    /// - Any other error that [`Fat::stat()`] returns.
    pub(crate) fn stat(&mut self, path: &FatResolvedPath) -> Result<Stat, Fat32Error> {
        let fat: &Fat = &self.fat;
        self.cache.stat(fat.epoch(), path.as_str(), || {
            fat.stat(path)
                .map(|info| Stat::new(info.size, info.is_dir, info.atime, info.mtime, info.ctime))
        })
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

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
