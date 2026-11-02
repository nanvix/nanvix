// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Path sandboxing for the host filesystem daemon.
//!
//! Ensures all guest-requested paths resolve within the configured root directory.
//! Rejects path traversal attacks and symlinks that escape the sandbox.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
use self::windows::WindowsPath;
#[cfg(windows)]
use ::fs_core::path::{
    walk,
    FinalComponent,
    WalkError,
};
use ::hostfs_api::HostResolvedPath;
use std::{
    io::{
        self,
        ErrorKind,
    },
    path::{
        Component,
        Path,
        PathBuf,
    },
};

/// Maximum symlink expansions during one Windows path walk.
#[cfg(windows)]
const MAX_SYMLINK_EXPANSIONS: usize = 40;

/// Windows reports cyclic symlink traversal with this error; the hostfs error mapper preserves it.
#[cfg(windows)]
const ERROR_CANT_RESOLVE_FILENAME: i32 = 1921;

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

    /// Resolves a checked host-relative path to an absolute host path within the sandbox.
    ///
    /// The value checks syntax only, not containment. Components and symlinks are interpreted
    /// here against the host root; a leading `mnt/` is an ordinary directory name.
    ///
    /// Returns a permission error for sandbox rejection and preserves filesystem errors such as
    /// missing ancestors and symlink loops.
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
    pub fn resolve(&self, relative_path: &HostResolvedPath) -> io::Result<PathBuf> {
        // Strip leading '/' — wire paths are relative to the mount point.
        let cleaned = relative_path.as_str().trim_start_matches('/');

        // Resolve intermediate symlinks before interpreting parent components on Windows.
        let candidate = self.candidate(cleaned, true)?;

        // Try to canonicalize directly (handles existing files and symlink resolution).
        // Fall back to parent canonicalization for files that don't exist yet (e.g., create).
        let resolved: PathBuf = match candidate.canonicalize() {
            Ok(p) => p,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                // Only a missing final entry is eligible for creation, not a traversal error.
                let parent = candidate.parent().ok_or(ErrorKind::PermissionDenied)?;
                let parent_resolved = parent.canonicalize()?;
                let file_name = candidate.file_name().ok_or(ErrorKind::PermissionDenied)?;
                parent_resolved.join(file_name)
            },
            Err(error) => return Err(error),
        };

        // Verify the resolved path is within the sandbox root.
        if resolved.starts_with(&self.root) {
            Ok(resolved)
        } else {
            Err(ErrorKind::PermissionDenied.into())
        }
    }

    /// Joins a host-relative path without discarding symlink-sensitive parent components.
    /// Containment is checked by the caller after resolution.
    fn candidate(&self, relative: &str, follow_final: bool) -> io::Result<PathBuf> {
        #[cfg(not(windows))]
        {
            let _ = follow_final;
            Ok(self.root.join(relative))
        }
        #[cfg(windows)]
        {
            let path = Path::new(relative);
            if path
                .components()
                .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
            {
                return Err(ErrorKind::PermissionDenied.into());
            }
            let mut adapter = WindowsPath::new(&self.root, path);
            let policy = if follow_final {
                FinalComponent::FollowOrMissing
            } else {
                FinalComponent::LeaveUninspected
            };
            walk(&mut adapter, policy, MAX_SYMLINK_EXPANSIONS).map_err(|error| match error {
                WalkError::Backend(error) => error,
                WalkError::TooManySymlinks => {
                    io::Error::from_raw_os_error(ERROR_CANT_RESOLVE_FILENAME)
                },
                WalkError::NotDirectory => ErrorKind::NotADirectory.into(),
            })?;
            Ok(adapter.into_path())
        }
    }

    /// Anchors a stored Windows target without consulting process-global per-drive cwd state.
    #[cfg(windows)]
    fn windows_target_base(base: &Path, target: &Path) -> io::Result<PathBuf> {
        if target.is_absolute() {
            return Ok(target
                .components()
                .take_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
                .collect());
        }
        if matches!(target.components().next(), Some(Component::Prefix(_))) {
            // `C:foo` is drive-relative, unlike `C:\\foo` or `\\foo`. Do not guess its cwd.
            return Err(ErrorKind::InvalidInput.into());
        }
        if target.has_root() {
            // `\\assets\\file` inherits the link's drive, or its server/share on a UNC volume.
            // Preserve the prefix rather than using the daemon's current drive.
            if !base.is_absolute() {
                return Err(ErrorKind::InvalidInput.into());
            }
            return Ok(base.components().take(2).collect());
        }
        Ok(base.to_path_buf())
    }

    /// Returns the sandbox root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    ///
    /// # Description
    ///
    /// Resolves a checked host-relative path to an absolute host path *without* following the
    /// final path component.
    ///
    /// Behaves like [`Self::resolve`] for every component except the last: the parent
    /// directory is canonicalized (and must exist and lie within the sandbox), and the
    /// unmodified final component is appended. This is the correct resolution mode for
    /// operations that must act on a symbolic link itself rather than on its target —
    /// `lstat`, `readlink`, `unlink` on a link, and `symlink` (creating a link must not
    /// follow any pre-existing link at the destination path).
    ///
    /// Bare names (no parent separator) resolve against the sandbox root.
    ///
    /// # Parameters
    ///
    /// - `relative_path`: A decoded path relative to the sandbox root. Leading `/` characters
    ///   are removed as for [`Self::resolve`]; the guest mount prefix is not stripped again.
    ///
    /// # Symlink TOCTOU
    ///
    /// The same TOCTOU caveat as [`Self::resolve`] applies: after the parent is
    /// canonicalized, an attacker swapping a symlink under the parent could still
    /// influence subsequent operations on the returned path. Closing that gap requires
    /// `openat()`-based dirfd operations.
    ///
    pub fn resolve_nofollow(&self, relative_path: &HostResolvedPath) -> io::Result<PathBuf> {
        let cleaned = relative_path.as_str().trim_start_matches('/');
        if cleaned.is_empty() {
            // Refers to the sandbox root itself; resolve normally.
            return Ok(self.root.clone());
        }
        // Check the raw final component before the Windows walk consumes any parent components.
        let last_component = Path::new(cleaned)
            .components()
            .next_back()
            .ok_or(ErrorKind::PermissionDenied)?;
        if !matches!(last_component, Component::Normal(_)) {
            return Err(ErrorKind::PermissionDenied.into());
        }
        let candidate = self.candidate(cleaned, false)?;
        let file_name = candidate.file_name().ok_or(ErrorKind::PermissionDenied)?;
        let parent = candidate.parent().ok_or(ErrorKind::PermissionDenied)?;
        let parent_resolved = parent.canonicalize()?;
        if !parent_resolved.starts_with(&self.root) {
            return Err(ErrorKind::PermissionDenied.into());
        }
        Ok(parent_resolved.join(file_name))
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests;
