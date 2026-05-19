// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Path sandboxing for the host filesystem daemon.
//!
//! Ensures all guest-requested paths resolve within the configured root directory.
//! Rejects path traversal attacks and symlinks that escape the sandbox.

use std::{
    io,
    path::{
        Path,
        PathBuf,
    },
};

/// A sandbox that constrains all filesystem operations to a root directory.
pub struct Sandbox {
    /// The absolute path of the root directory on the host.
    root: PathBuf,
}

impl Sandbox {
    /// Creates a new sandbox rooted at the given directory.
    ///
    /// Returns an error if the root directory does not exist, is not a directory,
    /// or cannot be canonicalized.
    pub fn new(root: PathBuf) -> io::Result<Self> {
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!("sandbox root is not an existing directory: {:?}", root),
            ));
        }
        let root: PathBuf = root.canonicalize()?;
        Ok(Self { root })
    }

    /// Resolves a guest-relative path to an absolute host path within the sandbox.
    ///
    /// Returns `None` if the resolved path escapes the sandbox root (path traversal).
    ///
    /// This avoids TOCTOU races by attempting `canonicalize()` directly rather than
    /// branching on `exists()`. If canonicalization fails (e.g., file not yet created),
    /// the parent directory is canonicalized instead and the filename is appended.
    ///
    /// # Symlink TOCTOU Limitation
    ///
    /// When the full path does not exist, only the parent directory is canonicalized.
    /// The final filename component is appended unchecked. If an attacker creates a
    /// symlink at that name between the `resolve()` call and the actual filesystem
    /// operation, the symlink target could escape the sandbox. Fully closing this gap
    /// requires opening the parent with `O_NOFOLLOW`-style flags and using `openat()`
    /// relative to that handle, which is platform-specific (Unix `O_NOFOLLOW` / Windows
    /// `FILE_FLAG_OPEN_REPARSE_POINT`) and out of scope for this PR.
    ///
    /// TODO(#sandbox-toctou): use `openat()` with `O_NOFOLLOW` to eliminate the
    /// symlink TOCTOU window for non-existent paths.
    pub fn resolve(&self, relative_path: &str) -> Option<PathBuf> {
        // Strip leading '/' — guest paths are relative to the mount point.
        let cleaned: &str = relative_path.trim_start_matches('/');

        // Join with root and canonicalize to resolve `.` and `..`.
        let candidate: PathBuf = self.root.join(cleaned);

        // Try to canonicalize directly (handles existing files and symlink resolution).
        // Fall back to parent canonicalization for files that don't exist yet (e.g., create).
        let resolved: PathBuf = match candidate.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Canonicalize the parent directory (must exist).
                let parent: &Path = candidate.parent()?;
                let parent_resolved: PathBuf = parent.canonicalize().ok()?;
                let file_name: &std::ffi::OsStr = candidate.file_name()?;
                parent_resolved.join(file_name)
            },
        };

        // Verify the resolved path is within the sandbox root.
        if resolved.starts_with(&self.root) {
            Some(resolved)
        } else {
            None
        }
    }

    /// Returns the sandbox root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }
}
