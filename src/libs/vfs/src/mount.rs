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
//! - A current working directory for relative path resolution

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::{
    string::String,
    vec::Vec,
};
use ::fat32::{
    Fat,
    Fat32Error,
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
    /// A new [`Mount`], or [`Fat32Error::InvalidPath`] if `path` doesn't start
    /// with "/".
    pub fn new(path: String, fat: Fat) -> Result<Self, Fat32Error> {
        if !path.starts_with('/') {
            return Err(Fat32Error::InvalidPath);
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
    /// Returns [`Fat32Error::AlreadyExists`] if a mount already exists at the
    /// same path.
    pub fn add_mount(&mut self, mount: Mount) -> Result<(), Fat32Error> {
        if self.mounts.iter().any(|m| m.path == mount.path) {
            return Err(Fat32Error::AlreadyExists);
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
    /// Returns [`Fat32Error::NotFound`] if no mount exists at this path.
    pub fn remove_mount(&mut self, path: &str) -> Result<Mount, Fat32Error> {
        let pos: usize = self
            .mounts
            .iter()
            .position(|m| m.path == path)
            .ok_or(Fat32Error::NotFound)?;

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
    /// - [`Fat32Error::InvalidPath`] if the path is malformed.
    /// - [`Fat32Error::NotFound`] if no mount handles this path.
    pub fn set_cwd(&mut self, path: &str) -> Result<(), Fat32Error> {
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
    /// Returns [`Fat32Error::InvalidPath`] if the path is empty or contains
    /// invalid sequences (e.g., too many `..`).
    pub fn normalize_path(&self, path: &str) -> Result<String, Fat32Error> {
        if path.is_empty() {
            return Err(Fat32Error::InvalidPath);
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
                        return Err(Fat32Error::InvalidPath);
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
    /// Returns [`Fat32Error::NotFound`] if no mount matches the path.
    pub fn resolve(&self, path: &str) -> Result<(usize, String), Fat32Error> {
        let normalized: String = self.normalize_path(path)?;

        for (idx, mount) in self.mounts.iter().enumerate() {
            if let Some(relative) = mount.matches(&normalized) {
                return Ok((idx, String::from(relative)));
            }
        }

        Err(Fat32Error::NotFound)
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

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // -- normalize_path tests ----------------------------------------------------

    /// Tests normalizing an absolute path.
    #[test]
    fn normalize_absolute_path() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/file.txt")
            .expect("should succeed");
        assert_eq!(result, "/data/file.txt");
    }

    /// Tests normalizing a relative path from root cwd.
    #[test]
    fn normalize_relative_from_root() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("file.txt").expect("should succeed");
        assert_eq!(result, "/file.txt");
    }

    /// Tests resolving "." in paths.
    #[test]
    fn normalize_dot() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/./file.txt")
            .expect("should succeed");
        assert_eq!(result, "/data/file.txt");
    }

    /// Tests resolving ".." in paths.
    #[test]
    fn normalize_dotdot() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/subdir/../file.txt")
            .expect("should succeed");
        assert_eq!(result, "/data/file.txt");
    }

    /// Tests resolving ".." at root yields root.
    #[test]
    fn normalize_dotdot_at_root() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("/data/..").expect("should succeed");
        assert_eq!(result, "/");
    }

    /// Tests that too many ".." returns an error.
    #[test]
    fn normalize_too_many_dotdot() {
        let vfs: Vfs = Vfs::new();
        let result = vfs.normalize_path("/data/../..");
        assert_eq!(result.unwrap_err(), Fat32Error::InvalidPath, "should fail with InvalidPath");
    }

    /// Tests that empty path returns an error.
    #[test]
    fn normalize_empty_path() {
        let vfs: Vfs = Vfs::new();
        let result = vfs.normalize_path("");
        assert_eq!(result.unwrap_err(), Fat32Error::InvalidPath, "empty path should fail");
    }

    /// Tests normalizing root path.
    #[test]
    fn normalize_root() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("/").expect("should succeed");
        assert_eq!(result, "/");
    }

    /// Tests trailing slashes are removed.
    #[test]
    fn normalize_trailing_slash() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("/data/subdir/").expect("should succeed");
        assert_eq!(result, "/data/subdir");
    }

    /// Tests relative path with non-root cwd.
    #[test]
    fn normalize_relative_with_cwd() {
        let mut vfs: Vfs = Vfs::new();
        vfs.cwd = String::from("/data");
        let result: String = vfs.normalize_path("file.txt").expect("should succeed");
        assert_eq!(result, "/data/file.txt");
    }

    // -- Mount::matches tests ----------------------------------------------------

    /// Helper: creates a Fat image in a heap buffer and returns a Mount.
    ///
    /// The returned `Vec<u8>` must be kept alive for the lifetime of the Mount.
    fn make_mount(mount_path: &str) -> (Mount, Vec<u8>) {
        use ::fat32::{
            Fat,
            RawMemoryStorage,
        };

        let size: usize = 64 * 1024;
        let mut buf: Vec<u8> = alloc::vec![0u8; size];
        let ptr: *mut u8 = buf.as_mut_ptr();

        // Format the buffer as FAT.
        let mut storage: RawMemoryStorage =
            unsafe { RawMemoryStorage::new(ptr, size).expect("valid storage") };
        let options = ::fatfs::FormatVolumeOptions::new();
        ::fatfs::format_volume(&mut storage, options).expect("format should succeed");

        // Create a Fat from the formatted buffer.
        let fat: Fat = unsafe { Fat::from_memory(ptr, size).expect("valid fat") };
        let mount: Mount = Mount::new(String::from(mount_path), fat).expect("valid mount");
        (mount, buf)
    }

    /// Tests that Mount::matches returns empty string for exact path match.
    #[test]
    fn mount_matches_exact() {
        let (mount, _buf) = make_mount("/data");
        assert_eq!(mount.matches("/data"), Some(""), "exact match should return empty relative");
    }

    /// Tests that Mount::matches returns relative subpath.
    #[test]
    fn mount_matches_subpath() {
        let (mount, _buf) = make_mount("/data");
        assert_eq!(
            mount.matches("/data/file.txt"),
            Some("file.txt"),
            "subpath should return relative"
        );
    }

    /// Tests that Mount::matches returns None for non-matching path.
    #[test]
    fn mount_matches_no_match() {
        let (mount, _buf) = make_mount("/data");
        assert_eq!(mount.matches("/other"), None, "different prefix should not match");
    }

    /// Tests that Mount::matches does not match partial prefix (e.g., /data2).
    #[test]
    fn mount_matches_partial_prefix() {
        let (mount, _buf) = make_mount("/data");
        assert_eq!(
            mount.matches("/data2"),
            None,
            "partial prefix should not match (/data vs /data2)"
        );
    }

    /// Tests root mount matches everything.
    #[test]
    fn mount_matches_root_mount() {
        let (mount, _buf) = make_mount("/");
        assert_eq!(mount.matches("/anything"), Some("anything"), "root mount should match all");
        assert_eq!(mount.matches("/"), Some(""), "root mount should match root");
    }

    /// Tests Mount path validation.
    #[test]
    fn mount_rejects_relative_path() {
        let size: usize = 64 * 1024;
        let mut buf: Vec<u8> = alloc::vec![0u8; size];
        let ptr: *mut u8 = buf.as_mut_ptr();

        let mut storage: ::fat32::RawMemoryStorage =
            unsafe { ::fat32::RawMemoryStorage::new(ptr, size).expect("valid storage") };
        ::fatfs::format_volume(&mut storage, ::fatfs::FormatVolumeOptions::new())
            .expect("format should succeed");
        let fat: ::fat32::Fat = unsafe { ::fat32::Fat::from_memory(ptr, size).expect("valid fat") };

        let result = Mount::new(String::from("relative"), fat);
        match result {
            Err(e) => assert_eq!(e, Fat32Error::InvalidPath, "relative path should be rejected"),
            Ok(_) => panic!("Mount::new should reject relative paths"),
        }
    }

    // -- VFS add/remove mount tests ----------------------------------------------

    /// Tests adding a mount and resolving a path through it.
    #[test]
    fn add_mount_and_resolve() {
        let mut vfs: Vfs = Vfs::new();
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add_mount should succeed");

        let (idx, relative) = vfs
            .resolve("/data/file.txt")
            .expect("resolve should succeed");
        assert_eq!(idx, 0, "mount index should be 0");
        assert_eq!(relative, "file.txt", "relative path should be 'file.txt'");
    }

    /// Tests resolving the mount root returns empty relative path.
    #[test]
    fn resolve_mount_root() {
        let mut vfs: Vfs = Vfs::new();
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add_mount should succeed");

        let (_idx, relative) = vfs.resolve("/data").expect("resolve should succeed");
        assert_eq!(relative, "", "mount root should resolve to empty relative path");
    }

    /// Tests that duplicate mount paths are rejected.
    #[test]
    fn add_duplicate_mount_fails() {
        let mut vfs: Vfs = Vfs::new();
        let (mount1, _buf1) = make_mount("/data");
        let (mount2, _buf2) = make_mount("/data");
        vfs.add_mount(mount1).expect("first add should succeed");

        let err: Fat32Error = vfs.add_mount(mount2).map(|_| ()).expect_err("should fail");
        assert_eq!(err, Fat32Error::AlreadyExists, "duplicate mount should be rejected");
    }

    /// Tests removing a mount.
    #[test]
    fn remove_mount() {
        let mut vfs: Vfs = Vfs::new();
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add_mount should succeed");
        assert_eq!(vfs.mount_count(), 1, "should have 1 mount");

        vfs.remove_mount("/data")
            .expect("remove_mount should succeed");
        assert_eq!(vfs.mount_count(), 0, "should have 0 mounts after removal");
    }

    /// Tests removing a non-existent mount fails.
    #[test]
    fn remove_nonexistent_mount_fails() {
        let mut vfs: Vfs = Vfs::new();
        let err: Fat32Error = vfs
            .remove_mount("/nonexistent")
            .map(|_| ())
            .expect_err("should fail");
        assert_eq!(err, Fat32Error::NotFound, "should fail with NotFound");
    }

    /// Tests resolving when no mounts exist fails.
    #[test]
    fn resolve_no_mounts_fails() {
        let vfs: Vfs = Vfs::new();
        let result = vfs.resolve("/anything");
        assert_eq!(result.unwrap_err(), Fat32Error::NotFound, "should fail with NotFound");
    }

    /// Tests longest-prefix matching with nested mounts.
    #[test]
    fn longest_prefix_matching() {
        let mut vfs: Vfs = Vfs::new();
        let (mount_data, _buf1) = make_mount("/data");
        let (mount_sub, _buf2) = make_mount("/data/sub");
        vfs.add_mount(mount_data).expect("add /data should succeed");
        vfs.add_mount(mount_sub)
            .expect("add /data/sub should succeed");

        // /data/sub/file should resolve to the /data/sub mount.
        let (idx, relative) = vfs
            .resolve("/data/sub/file.txt")
            .expect("resolve should succeed");
        let mount_path: &str = vfs.get_mount(idx).expect("mount should exist").path();
        assert_eq!(mount_path, "/data/sub", "should match longer mount");
        assert_eq!(relative, "file.txt", "relative path within /data/sub");

        // /data/other should resolve to the /data mount.
        let (idx2, relative2) = vfs
            .resolve("/data/other.txt")
            .expect("resolve should succeed");
        let mount_path2: &str = vfs.get_mount(idx2).expect("mount should exist").path();
        assert_eq!(mount_path2, "/data", "should match /data mount");
        assert_eq!(relative2, "other.txt", "relative path within /data");
    }

    /// Tests setting and getting cwd.
    #[test]
    fn set_and_get_cwd() {
        let mut vfs: Vfs = Vfs::new();
        assert_eq!(vfs.cwd(), "/", "initial cwd should be /");

        // Need a mount for set_cwd to verify path exists.
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add_mount should succeed");

        vfs.set_cwd("/data").expect("set_cwd should succeed");
        assert_eq!(vfs.cwd(), "/data", "cwd should be updated");
    }

    /// Tests that set_cwd to "/" always works (even without mounts).
    #[test]
    fn set_cwd_root() {
        let mut vfs: Vfs = Vfs::new();
        vfs.set_cwd("/").expect("set_cwd to / should succeed");
        assert_eq!(vfs.cwd(), "/");
    }

    /// Tests Default trait implementation.
    #[test]
    fn default_vfs() {
        let vfs: Vfs = Vfs::default();
        assert_eq!(vfs.cwd(), "/");
        assert_eq!(vfs.mount_count(), 0);
    }
}
