// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Virtual filesystem layer.
//!
//! This module provides the VFS mount table and path resolution logic that
//! routes filesystem operations to the appropriate FAT backend.
//!
//! # Architecture
//!
//! The VFS maintains:
//! - A mount table mapping paths to FAT backends (sorted by path length for
//!   longest-prefix matching)
//! - A path-resolution cache keyed by normalized absolute paths
//!
//! Callers provide the current working directory when resolving relative paths.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    path_cache::PathCache,
    Mount,
};
use crate::path::AnchoredPath;
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::fat32::{
    Fat32Error,
    FatResolvedPath,
};

//==================================================================================================
// VFS Structure
//==================================================================================================

/// Internal result of resolving a path through the VFS mount table.
pub(crate) struct FatResolutionContext {
    /// Index of the selected mount.
    pub(crate) mount_index: usize,
    /// Normalized absolute path used as the cache key.
    pub(crate) normalized_absolute: String,
    /// Checked path relative to the selected FAT mount.
    pub(crate) fat_path: FatResolvedPath,
}

/// Virtual filesystem managing mounts and path resolution.
///
/// # Path Resolution
///
/// 1. Normalize the path (resolve `.`, `..`, make absolute using cwd)
/// 2. Search mounts in order (sorted by path length descending)
/// 3. Return first mount where path starts with mount.path
/// 4. Extract relative path by stripping mount prefix
pub struct Vfs {
    /// Mount table, sorted by path length descending for
    /// longest-prefix matching.
    mounts: Vec<Mount>,
    /// Cache mapping normalized paths to their resolution. Invalidated
    /// whenever the mount table changes.
    resolve_cache: PathCache,
}

//==================================================================================================
// VFS Implementations
//==================================================================================================

impl Vfs {
    /// Creates a new empty VFS.
    pub fn new() -> Self {
        Self {
            mounts: Vec::new(),
            resolve_cache: PathCache::new(),
        }
    }

    /// Adds a mount point.
    ///
    /// The mount is inserted at the correct position to maintain
    /// descending path length order.
    ///
    /// # Parameters
    ///
    /// - `mount`: The mount point to add.
    ///
    /// # Errors
    ///
    /// Returns [`Fat32Error::AlreadyExists`] if a mount already exists at the
    /// same path.
    pub fn add_mount(&mut self, mount: Mount) -> Result<(), Fat32Error> {
        if self
            .mounts
            .iter()
            .any(|candidate| candidate.path() == mount.path())
        {
            return Err(Fat32Error::AlreadyExists);
        }

        let pos: usize = self
            .mounts
            .iter()
            .position(|candidate| candidate.path().len() < mount.path().len())
            .unwrap_or(self.mounts.len());

        self.mounts.insert(pos, mount);
        // The mount table changed: cached resolutions (including mount
        // indices) may now be stale, so drop them.
        self.resolve_cache.clear();
        Ok(())
    }

    /// Removes a mount point.
    ///
    /// # Parameters
    ///
    /// - `path`: The mount path to remove.
    ///
    /// # Errors
    ///
    /// Returns [`Fat32Error::NotFound`] if no mount exists at this path.
    pub fn remove_mount(&mut self, path: &str) -> Result<Mount, Fat32Error> {
        let pos: usize = self
            .mounts
            .iter()
            .position(|mount| mount.path() == path)
            .ok_or(Fat32Error::NotFound)?;

        let removed: Mount = self.mounts.remove(pos);
        // The mount table changed: cached resolutions (including mount
        // indices) may now be stale, so drop them.
        self.resolve_cache.clear();
        Ok(removed)
    }

    /// Normalizes a path to an absolute path.
    ///
    /// - Resolves `.` (current directory)
    /// - Resolves `..` (parent directory)
    /// - Makes relative paths absolute using `cwd`
    /// - Removes trailing slashes (except for root)
    ///
    /// # Parameters
    ///
    /// - `path`: The path to normalize.
    /// - `cwd`: The absolute current working directory used to anchor relative paths.
    ///
    /// # Errors
    ///
    /// Returns [`Fat32Error::NotFound`] if the path is empty. Returns
    /// [`Fat32Error::InvalidPath`] if `path` contains an embedded null byte or a relative `path` is
    /// anchored to a `cwd` that is not absolute. `..` at the root clamps to the root, per POSIX.
    ///
    /// # References
    ///
    /// - [POSIX Base Definitions, Chapter 4 — Pathname Resolution](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html)
    /// - [POSIX open() — `ENOENT` for empty path](https://pubs.opengroup.org/onlinepubs/9799919799/functions/open.html)
    pub fn normalize_path(&self, path: &str, cwd: &str) -> Result<String, Fat32Error> {
        normalize_path(path, cwd)
    }

    /// Resolves a raw anchored path to a checked FAT-local path.
    ///
    /// The path is anchored once, lexically normalized, matched against the longest mount prefix,
    /// and checked by the FAT path constructor. Successful resolutions are cached by normalized
    /// absolute path; cache hits reconstruct the same checked proof.
    ///
    /// # Parameters
    ///
    /// - `path`: Raw path provenance to consume.
    ///
    /// # Returns
    ///
    /// A checked path relative to the selected FAT mount.
    ///
    /// # Errors
    ///
    /// Returns an anchoring error from [`AnchoredPath`], [`Fat32Error::NotFound`] if no mount
    /// matches, or [`Fat32Error::InvalidPath`] if the VFS produces an invalid FAT-local path.
    pub fn resolve(&mut self, path: AnchoredPath) -> Result<FatResolvedPath, Fat32Error> {
        let absolute: String = path.into_absolute()?;
        let normalized: String = normalize_anchored(&absolute);
        let context: FatResolutionContext = self.resolve_context(normalized)?;
        debug_assert!(context.normalized_absolute.starts_with('/'));
        Ok(context.fat_path)
    }

    /// Resolves a normalized absolute path for a local filesystem operation.
    pub(crate) fn resolve_normalized(
        &mut self,
        normalized: &str,
    ) -> Result<(usize, FatResolvedPath), Fat32Error> {
        let context: FatResolutionContext = self.resolve_context(String::from(normalized))?;
        Ok((context.mount_index, context.fat_path))
    }

    /// Resolves a normalized absolute path through the cache and mount table.
    fn resolve_context(&mut self, normalized: String) -> Result<FatResolutionContext, Fat32Error> {
        if let Some((mount_index, relative)) = self.resolve_cache.get(&normalized) {
            return Ok(FatResolutionContext {
                mount_index,
                normalized_absolute: normalized,
                fat_path: FatResolvedPath::new(relative)?,
            });
        }

        // Compute the mount match before mutating the cache to keep the borrows disjoint.
        let mut resolved: Option<(usize, String)> = None;
        for (mount_index, mount) in self.mounts.iter().enumerate() {
            if let Some(relative) = mount.matches(&normalized) {
                resolved = Some((mount_index, String::from(relative)));
                break;
            }
        }

        match resolved {
            Some((mount_index, relative)) => {
                let fat_path: FatResolvedPath = FatResolvedPath::new(relative)?;
                self.resolve_cache.insert(
                    normalized.clone(),
                    mount_index,
                    String::from(fat_path.as_str()),
                );
                Ok(FatResolutionContext {
                    mount_index,
                    normalized_absolute: normalized,
                    fat_path,
                })
            },
            None => Err(Fat32Error::NotFound),
        }
    }

    /// Gets a reference to a mount by index.
    #[inline]
    pub fn get_mount(&self, index: usize) -> Option<&Mount> {
        self.mounts.get(index)
    }

    /// Gets a mutable reference to a mount by index.
    #[inline]
    pub fn get_mount_mut(&mut self, index: usize) -> Option<&mut Mount> {
        self.mounts.get_mut(index)
    }

    /// Returns the number of mounts.
    #[inline]
    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }

    /// Iterates over all mounts.
    pub fn mounts(&self) -> impl Iterator<Item = &Mount> {
        self.mounts.iter()
    }
}

/// Anchors a path to an absolute working directory without normalizing its components.
pub(crate) fn anchor_path(path: &str, cwd: &str) -> Result<String, Fat32Error> {
    AnchoredPath::validate_raw(path)?;
    if path.starts_with('/') {
        Ok(String::from(path))
    } else if !cwd.starts_with('/') {
        Err(Fat32Error::InvalidPath)
    } else if cwd == "/" {
        Ok(alloc::format!("/{}", path))
    } else {
        Ok(alloc::format!("{}/{}", cwd, path))
    }
}

/// Normalizes a path after anchoring it to an absolute working directory.
pub(crate) fn normalize_path(path: &str, cwd: &str) -> Result<String, Fat32Error> {
    Ok(normalize_anchored(&anchor_path(path, cwd)?))
}

/// Lexically normalizes an anchored absolute path.
pub(crate) fn normalize_anchored(path: &str) -> String {
    normalize_components(path)
}

/// Lexically normalizes an absolute `path`.
///
/// Shared with [`Vfs::normalize_path`], which anchors relative paths against a
/// `cwd` before calling in.
fn normalize_components(path: &str) -> String {
    let mut components: Vec<&str> = Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => {},
            ".." => {
                components.pop();
            },
            other => {
                components.push(other);
            },
        }
    }

    if components.is_empty() {
        return String::from("/");
    }

    let mut result: String = String::new();
    for component in components {
        result.push('/');
        result.push_str(component);
    }
    result
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
