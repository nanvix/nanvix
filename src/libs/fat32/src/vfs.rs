// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

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
//! - A current working directory for relative path resolution

//==================================================================================================
// Imports
//==================================================================================================

use alloc::{
    string::String,
    vec::Vec,
};

use crate::{
    error::FsError,
    fat::Fat,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A single mount point in the VFS.
///
/// Associates an absolute path with a FAT filesystem backend.
pub struct Mount {
    /// Absolute path where this mount is rooted (e.g., "/data").
    ///
    /// Always starts with "/" and never ends with "/" (except for root "/").
    path: String,
    /// The FAT filesystem backend for this mount.
    fat: Fat,
}

//==================================================================================================
// Mount Implementations
//==================================================================================================

impl Mount {
    /// Creates a new mount point.
    ///
    /// # Parameters
    ///
    /// - `path`: Absolute mount path (must start with "/").
    /// - `fat`: The FAT filesystem backend.
    ///
    /// # Returns
    ///
    /// A new [`Mount`], or [`FsError::InvalidPath`] if `path` doesn't start
    /// with "/".
    pub fn new(path: String, fat: Fat) -> Result<Self, FsError> {
        if !path.starts_with('/') {
            return Err(FsError::InvalidPath);
        }
        Ok(Self { path, fat })
    }

    /// Returns the mount path.
    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns a reference to the FAT backend.
    #[inline]
    pub fn fat(&self) -> &Fat {
        &self.fat
    }

    /// Returns a mutable reference to the FAT backend.
    #[inline]
    pub fn fat_mut(&mut self) -> &mut Fat {
        &mut self.fat
    }

    /// Checks if the given path is under this mount point.
    ///
    /// # Parameters
    ///
    /// - `path`: The absolute path to test.
    ///
    /// # Returns
    ///
    /// `Some(relative_path)` if the path matches this mount, `None` otherwise.
    pub fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        if self.path == "/" {
            Some(path.strip_prefix('/').unwrap_or(path))
        } else if path == self.path {
            Some("")
        } else if path.starts_with(&self.path) {
            let rest: &str = &path[self.path.len()..];
            if let Some(stripped) = rest.strip_prefix('/') {
                Some(stripped)
            } else {
                None
            }
        } else {
            None
        }
    }
}

//==================================================================================================
// VFS Structure
//==================================================================================================

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
    /// Current working directory (always absolute, never ends with "/").
    cwd: String,
}

//==================================================================================================
// VFS Implementations
//==================================================================================================

impl Vfs {
    /// Creates a new empty VFS with cwd set to "/".
    pub fn new() -> Self {
        Self {
            mounts: Vec::new(),
            cwd: String::from("/"),
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
    /// Returns [`FsError::AlreadyExists`] if a mount already exists at the
    /// same path.
    pub fn add_mount(&mut self, mount: Mount) -> Result<(), FsError> {
        if self.mounts.iter().any(|m| m.path == mount.path) {
            return Err(FsError::AlreadyExists);
        }

        let pos: usize = self
            .mounts
            .iter()
            .position(|m| m.path.len() < mount.path.len())
            .unwrap_or(self.mounts.len());

        self.mounts.insert(pos, mount);
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
    /// Returns [`FsError::NotFound`] if no mount exists at this path.
    pub fn remove_mount(&mut self, path: &str) -> Result<Mount, FsError> {
        let pos: usize = self
            .mounts
            .iter()
            .position(|m| m.path == path)
            .ok_or(FsError::NotFound)?;

        Ok(self.mounts.remove(pos))
    }

    /// Gets the current working directory.
    #[inline]
    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    /// Changes the current working directory.
    ///
    /// # Parameters
    ///
    /// - `path`: The new working directory path.
    ///
    /// # Errors
    ///
    /// - [`FsError::InvalidPath`] if the path is malformed.
    /// - [`FsError::NotFound`] if no mount handles this path.
    pub fn set_cwd(&mut self, path: &str) -> Result<(), FsError> {
        let normalized: String = self.normalize_path(path)?;

        if !normalized.is_empty() && normalized != "/" {
            let _ = self.resolve(&normalized)?;
        }

        self.cwd = normalized;
        Ok(())
    }

    /// Normalizes a path to an absolute path.
    ///
    /// - Resolves `.` (current directory)
    /// - Resolves `..` (parent directory)
    /// - Makes relative paths absolute using cwd
    /// - Removes trailing slashes (except for root)
    ///
    /// # Parameters
    ///
    /// - `path`: The path to normalize.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::InvalidPath`] if the path is empty or contains
    /// invalid sequences (e.g., too many `..`).
    pub fn normalize_path(&self, path: &str) -> Result<String, FsError> {
        if path.is_empty() {
            return Err(FsError::InvalidPath);
        }

        let abs_path: String = if path.starts_with('/') {
            String::from(path)
        } else if self.cwd == "/" {
            alloc::format!("/{}", path)
        } else {
            alloc::format!("{}/{}", self.cwd, path)
        };

        let mut components: Vec<&str> = Vec::new();

        for component in abs_path.split('/') {
            match component {
                "" | "." => {},
                ".." => {
                    if components.pop().is_none() {
                        return Err(FsError::InvalidPath);
                    }
                },
                other => {
                    components.push(other);
                },
            }
        }

        if components.is_empty() {
            Ok(String::from("/"))
        } else {
            let mut result: String = String::new();
            for component in components {
                result.push('/');
                result.push_str(component);
            }
            Ok(result)
        }
    }

    /// Resolves a path to a mount and relative path within that mount.
    ///
    /// Uses longest-prefix matching to find the best mount.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to resolve.
    ///
    /// # Returns
    ///
    /// A tuple of `(mount_index, relative_path)`.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::NotFound`] if no mount matches the path.
    pub fn resolve(&self, path: &str) -> Result<(usize, String), FsError> {
        let normalized: String = self.normalize_path(path)?;

        for (idx, mount) in self.mounts.iter().enumerate() {
            if let Some(relative) = mount.matches(&normalized) {
                return Ok((idx, String::from(relative)));
            }
        }

        Err(FsError::NotFound)
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

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}
