// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::sysapi::fcntl::atflags::AT_FDCWD;

impl Vfs {
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

    /// Returns the number of entries currently held by the path-resolution
    /// cache. Test-only helper used to assert caching and eviction behavior.
    pub(crate) fn resolve_cache_len(&self) -> usize {
        self.resolve_cache.entry_count()
    }
}

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
    AnchoredPath::new(-1, String::from(path)).expect("valid raw path should be accepted")
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
    assert_eq!(mount.matches("/data/file.txt"), Some("file.txt"), "subpath should return relative");
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
    assert_eq!(mount.matches("/data2"), None, "partial prefix should not match (/data vs /data2)");
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

#[test]
fn resolution_rejects_null_before_normalization() {
    let mut vfs: Vfs = Vfs::new();
    let (mount, _buffer) = make_mount("/data");
    vfs.add_mount(mount).expect("add data mount");

    let path: &str = "/data/invalid\0/../file";
    assert_eq!(AnchoredPath::new(-1, String::from(path)), Err(Fat32Error::InvalidPath),);
    assert_eq!(vfs.resolve_legacy(path, "/"), Err(Fat32Error::InvalidPath));
    assert_eq!(vfs.resolve_cache_len(), 0);
}

#[test]
fn current_directory_resolution_does_not_relock_vfs_state() {
    if !crate::state::is_initialized() {
        let _result: Result<(), Fat32Error> = crate::state::init();
    }

    crate::state::with_vfs_mut(|vfs: &mut Vfs| {
        let path: AnchoredPath =
            AnchoredPath::new(AT_FDCWD, String::from("typed-resolution-lock-check"))
                .expect("valid raw path should be accepted");
        let _resolution: Result<FatResolvedPath, Fat32Error> = vfs.resolve(path);
        Ok(())
    })
    .expect("typed resolution should not recursively lock VFS state");
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
