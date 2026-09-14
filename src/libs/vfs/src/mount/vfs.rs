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
use crate::path::{
    AnchoredPath,
    ResolvedPath,
};
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
    /// [`Fat32Error::InvalidPath`] if a relative `path` is anchored to a `cwd` that is not
    /// absolute. `..` at the root clamps to the root, per POSIX.
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

    /// Resolves a path for consumers awaiting migration to [`AnchoredPath`].
    ///
    /// TODO (#3101): Remove this compatibility projection after filesystem consumers accept typed
    /// resolution results.
    pub(crate) fn resolve_legacy(
        &mut self,
        path: &str,
        cwd: &str,
    ) -> Result<(usize, String), Fat32Error> {
        let normalized: String = self.normalize_path(path, cwd)?;
        let context: FatResolutionContext = self.resolve_context(normalized)?;
        Ok((context.mount_index, context.fat_path.into_string()))
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

    /// Returns the number of entries currently held by the path-resolution
    /// cache. Test-only helper used to assert caching and eviction behavior.
    #[cfg(test)]
    pub(crate) fn resolve_cache_len(&self) -> usize {
        self.resolve_cache.entry_count()
    }
}

/// Anchors a path to an absolute working directory without normalizing its components.
pub(crate) fn anchor_path(path: &str, cwd: &str) -> Result<String, Fat32Error> {
    if path.is_empty() {
        return Err(Fat32Error::NotFound);
    }
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

/// Lexically normalizes a resolved path.
///
/// Resolves `.` and `..`, collapses repeated separators, and drops trailing
/// slashes. Never fails: [`ResolvedPath`] is absolute by construction, and `..`
/// at the root clamps to the root, as POSIX defines `/..` to be `/`.
pub(crate) fn normalize_absolute(path: &ResolvedPath) -> String {
    normalize_anchored(path.as_str())
}

/// Lexically normalizes an anchored absolute path.
pub(crate) fn normalize_anchored(path: &str) -> String {
    normalize_components(path)
}

/// Lexically normalizes an absolute `path`.
///
/// Shared with [`Vfs::normalize_path`], which anchors relative paths against a
/// `cwd` before calling in and so cannot hand over a [`ResolvedPath`].
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
mod tests {
    use super::*;

    // -- normalize_path tests ----------------------------------------------------

    /// Tests normalizing an absolute path.
    #[test]
    fn normalize_absolute_path() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/file.txt", "/")
            .expect("should succeed");
        assert_eq!(result, "/data/file.txt");
    }

    /// Tests normalizing a relative path from root cwd.
    #[test]
    fn normalize_relative_from_root() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("file.txt", "/").expect("should succeed");
        assert_eq!(result, "/file.txt");
    }

    /// Tests resolving "." in paths.
    #[test]
    fn normalize_dot() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/./file.txt", "/")
            .expect("should succeed");
        assert_eq!(result, "/data/file.txt");
    }

    /// Tests resolving ".." in paths.
    #[test]
    fn normalize_dotdot() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/subdir/../file.txt", "/")
            .expect("should succeed");
        assert_eq!(result, "/data/file.txt");
    }

    /// Tests resolving ".." at root yields root.
    #[test]
    fn normalize_dotdot_at_root() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("/data/..", "/").expect("should succeed");
        assert_eq!(result, "/");
    }

    /// Tests that ".." past the root clamps to the root, per POSIX.
    #[test]
    fn normalize_dotdot_past_root_clamps() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/../..", "/")
            .expect("should succeed");
        assert_eq!(result, "/", ".. past root should clamp to root");
    }

    /// Tests that empty path returns an error.
    #[test]
    fn normalize_empty_path() {
        let vfs: Vfs = Vfs::new();
        let result = vfs.normalize_path("", "/");
        assert_eq!(result.unwrap_err(), Fat32Error::NotFound, "empty path should fail");
    }

    /// Tests normalizing root path.
    #[test]
    fn normalize_root() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("/", "/").expect("should succeed");
        assert_eq!(result, "/");
    }

    /// Tests trailing slashes are removed.
    #[test]
    fn normalize_trailing_slash() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/subdir/", "/")
            .expect("should succeed");
        assert_eq!(result, "/data/subdir");
    }

    /// Tests relative path with non-root cwd.
    #[test]
    fn normalize_relative_with_cwd() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("file.txt", "/data")
            .expect("should succeed");
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
        let mount: Mount = Mount::new(String::from(mount_path), fat, false).expect("valid mount");
        (mount, buf)
    }

    /// Creates raw provenance for an absolute path, which ignores the supplied descriptor.
    fn anchored(path: &str) -> AnchoredPath {
        AnchoredPath::new(-1, String::from(path))
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

        let result = Mount::new(String::from("relative"), fat, false);
        match result {
            Err(e) => assert_eq!(e, Fat32Error::InvalidPath, "relative path should be rejected"),
            Ok(_) => panic!("Mount::new should reject relative paths"),
        }
    }

    // -- VFS add/remove mount tests ----------------------------------------------

    /// Tests adding a mount and resolving a checked path through it.
    #[test]
    fn add_mount_and_resolve() {
        let mut vfs: Vfs = Vfs::new();
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add_mount should succeed");

        let relative: FatResolvedPath = vfs
            .resolve(anchored("/data/file.txt"))
            .expect("resolve should succeed");
        assert_eq!(relative.as_str(), "file.txt");
    }

    /// Tests resolving exact mount roots through the checked API.
    #[test]
    fn resolve_mount_root() {
        for mount_path in ["/", "/data"] {
            let mut vfs: Vfs = Vfs::new();
            let (mount, _buf) = make_mount(mount_path);
            vfs.add_mount(mount).expect("add_mount should succeed");

            let relative: FatResolvedPath = vfs
                .resolve(anchored(mount_path))
                .expect("mount root should resolve");
            assert_eq!(relative.as_str(), "", "mount root should produce an empty FAT path");
        }
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
        let mut vfs: Vfs = Vfs::new();
        let result: Result<FatResolvedPath, Fat32Error> = vfs.resolve(anchored("/anything"));
        assert_eq!(result, Err(Fat32Error::NotFound));
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

        let nested: FatResolvedPath = vfs
            .resolve(anchored("/data/sub/file.txt"))
            .expect("nested path should resolve");
        assert_eq!(nested.as_str(), "file.txt", "longest mount prefix should be selected");

        let parent: FatResolvedPath = vfs
            .resolve(anchored("/data/other.txt"))
            .expect("parent path should resolve");
        assert_eq!(parent.as_str(), "other.txt");
    }

    /// Tests lexical normalization and mount-boundary matching through the checked API.
    #[test]
    fn checked_resolution_normalizes_and_respects_boundaries() {
        let mut vfs: Vfs = Vfs::new();
        let (root, _root_buf) = make_mount("/");
        let (data, _data_buf) = make_mount("/data");
        vfs.add_mount(root).expect("add root mount");
        vfs.add_mount(data).expect("add data mount");

        let normalized: FatResolvedPath = vfs
            .resolve(anchored("/data//directory/./child/../file"))
            .expect("normalized path should resolve");
        assert_eq!(normalized.as_str(), "directory/file");

        let false_prefix: FatResolvedPath = vfs
            .resolve(anchored("/data2/file"))
            .expect("false prefix should fall through to root mount");
        assert_eq!(false_prefix.as_str(), "data2/file");
    }

    /// Tests that the temporary projection delegates to the checked resolver core.
    #[test]
    fn legacy_projection_matches_checked_resolution() {
        let mut vfs: Vfs = Vfs::new();
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add data mount");

        let checked: FatResolvedPath = vfs
            .resolve(anchored("/data/./directory/../file"))
            .expect("checked path should resolve");
        let context: FatResolutionContext = vfs
            .resolve_context(String::from("/data/file"))
            .expect("context should resolve");
        let (_mount_index, legacy): (usize, String) = vfs
            .resolve_legacy("/data/./directory/../file", "/")
            .expect("legacy path should resolve");

        assert_eq!(checked.as_str(), legacy);
        assert_eq!(context.normalized_absolute, "/data/file");
        assert_eq!(context.fat_path, checked);
    }

    // -- Path-resolution cache tests ---------------------------------------------

    /// Tests that cache misses and hits return equivalent checked proofs.
    #[test]
    fn resolve_populates_cache() {
        let mut vfs: Vfs = Vfs::new();
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add_mount should succeed");

        assert_eq!(vfs.resolve_cache_len(), 0, "cache starts empty");

        let first: FatResolvedPath = vfs
            .resolve(anchored("/data/file.txt"))
            .expect("resolve should succeed");
        assert_eq!(vfs.resolve_cache_len(), 1, "resolve should cache the result");

        let second: FatResolvedPath = vfs
            .resolve(anchored("/data/file.txt"))
            .expect("cached resolve should succeed");
        assert_eq!(first, second, "cache hit proof must match the cache miss proof");
        assert_eq!(vfs.resolve_cache_len(), 1, "repeat lookup must not grow the cache");
    }

    /// Tests that resolving via the cache still anchors relative paths to the
    /// supplied cwd (cache is keyed by the normalized absolute path).
    #[test]
    fn resolve_cache_respects_cwd() {
        let mut vfs: Vfs = Vfs::new();
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add_mount should succeed");

        let (_idx, relative_from_abs) = vfs
            .resolve_legacy("/data/file.txt", "/")
            .expect("absolute resolve should succeed");
        let (_idx2, relative_from_cwd) = vfs
            .resolve_legacy("file.txt", "/data")
            .expect("relative resolve should succeed");
        assert_eq!(
            relative_from_abs, relative_from_cwd,
            "relative path anchored to cwd must match the absolute form"
        );
    }

    /// Tests that adding a mount invalidates stale cache entries so that
    /// subsequent resolutions return the correct mount index.
    #[test]
    fn add_mount_invalidates_cache() {
        let mut vfs: Vfs = Vfs::new();
        let (mount_data, _buf1) = make_mount("/data");
        vfs.add_mount(mount_data).expect("add /data should succeed");

        // Cache the resolution while /data is the only mount (index 0).
        let (idx, _rel) = vfs
            .resolve_legacy("/data/file.txt", "/")
            .expect("resolve should succeed");
        assert_eq!(idx, 0, "/data should be at index 0 initially");

        // Adding a longer mount path shifts /data to a higher index.
        let (mount_longer, _buf2) = make_mount("/datadir");
        vfs.add_mount(mount_longer)
            .expect("add /datadir should succeed");
        assert_eq!(vfs.resolve_cache_len(), 0, "add_mount must clear the cache");

        let (idx2, rel2) = vfs
            .resolve_legacy("/data/file.txt", "/")
            .expect("resolve should succeed");
        let mount_path: &str = vfs.get_mount(idx2).expect("mount should exist").path();
        assert_eq!(mount_path, "/data", "resolution must still point at /data after invalidation");
        assert_eq!(rel2, "file.txt", "relative path within /data");
    }

    /// Tests that removing a mount invalidates the cache.
    #[test]
    fn remove_mount_invalidates_cache() {
        let mut vfs: Vfs = Vfs::new();
        let (mount, _buf) = make_mount("/data");
        vfs.add_mount(mount).expect("add_mount should succeed");

        let _ = vfs
            .resolve_legacy("/data/file.txt", "/")
            .expect("resolve should succeed");
        assert_eq!(vfs.resolve_cache_len(), 1, "resolve should cache the result");

        vfs.remove_mount("/data")
            .expect("remove_mount should succeed");
        assert_eq!(vfs.resolve_cache_len(), 0, "remove_mount must clear the cache");

        let result = vfs.resolve_legacy("/data/file.txt", "/");
        assert_eq!(
            result.unwrap_err(),
            Fat32Error::NotFound,
            "resolution must fail once the mount is gone"
        );
    }

    /// Tests Default trait implementation.
    #[test]
    fn default_vfs() {
        let vfs: Vfs = Vfs::default();
        assert_eq!(vfs.mount_count(), 0);
    }

    // -- Read-only mount tests ---------------------------------------------------

    /// Helper: creates a Fat image and returns a read-only Mount.
    fn make_readonly_mount(mount_path: &str) -> (Mount, Vec<u8>) {
        use ::fat32::{
            Fat,
            RawMemoryStorage,
        };

        let size: usize = 64 * 1024;
        let mut buf: Vec<u8> = alloc::vec![0u8; size];
        let ptr: *mut u8 = buf.as_mut_ptr();

        let mut storage: RawMemoryStorage =
            unsafe { RawMemoryStorage::new(ptr, size).expect("valid storage") };
        let options = ::fatfs::FormatVolumeOptions::new();
        ::fatfs::format_volume(&mut storage, options).expect("format should succeed");

        let fat: Fat = unsafe { Fat::from_memory(ptr, size).expect("valid fat") };
        let mount: Mount = Mount::new(String::from(mount_path), fat, true).expect("valid mount");
        (mount, buf)
    }

    /// Tests that a writable mount returns readonly() == false.
    #[test]
    fn mount_writable_flag() {
        let (mount, _buf) = make_mount("/data");
        assert!(!mount.readonly(), "writable mount should return false");
    }

    /// Tests that a read-only mount returns readonly() == true.
    #[test]
    fn mount_readonly_flag() {
        let (mount, _buf) = make_readonly_mount("/data");
        assert!(mount.readonly(), "read-only mount should return true");
    }

    // -- Tilde is no longer expanded server-side ---------------------------------

    /// Tests that "~" is treated as a relative path (no expansion).
    #[test]
    fn normalize_tilde_is_relative() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("~", "/").expect("should succeed");
        assert_eq!(result, "/~", "bare ~ should be treated as relative segment");
    }

    /// Tests that "~/foo" is treated as a relative path (no expansion).
    #[test]
    fn normalize_tilde_subpath_is_relative() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("~/foo", "/").expect("should succeed");
        assert_eq!(result, "/~/foo", "~/foo should be treated as relative path");
    }

    /// Tests that paths not starting with "~" are unaffected.
    #[test]
    fn normalize_no_tilde() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs
            .normalize_path("/data/file.txt", "/")
            .expect("should succeed");
        assert_eq!(result, "/data/file.txt");
    }

    /// Tests that "~user" is treated as a relative path.
    #[test]
    fn normalize_tilde_user_not_expanded() {
        let vfs: Vfs = Vfs::new();
        let result: String = vfs.normalize_path("~other", "/").expect("should succeed");
        assert_eq!(result, "/~other", "~user should not be expanded");
    }
}
