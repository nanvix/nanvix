// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! VFS path provenance and anchoring.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    descriptor::VfsFileHandle,
    fd::entry_arc,
    process::{
        current_cwd,
        OpenFile,
    },
};
use ::alloc::{
    format,
    string::String,
};
use ::fat32::Fat32Error;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Structures
//==================================================================================================

/// A raw directory descriptor and path pair supplied at a system-call boundary.
///
/// Construction preserves provenance without validating, joining, or normalizing the path. A
/// backend resolver consumes this value and applies the appropriate anchoring policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredPath {
    /// Directory descriptor used to anchor a relative path.
    dirfd: c_int,
    /// Raw path supplied by the caller.
    path: String,
}

impl AnchoredPath {
    /// Records a raw directory descriptor and path pair.
    ///
    /// # Parameters
    ///
    /// - `dirfd`: Directory descriptor used for relative paths.
    /// - `path`: Raw path supplied by the caller.
    ///
    /// # Returns
    ///
    /// A path value that preserves the supplied pair without validation or transformation.
    pub fn new(dirfd: c_int, path: String) -> Self {
        Self { dirfd, path }
    }

    /// Returns the directory descriptor supplied by the caller.
    pub fn dirfd(&self) -> c_int {
        self.dirfd
    }

    /// Returns the raw path supplied by the caller.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Consumes this value and anchors its path in the VFS namespace.
    pub(crate) fn into_absolute(self) -> Result<String, Fat32Error> {
        anchor_with(
            self,
            || {
                if !crate::state::is_initialized() {
                    return Err(Fat32Error::InvalidArgument);
                }
                Ok(current_cwd())
            },
            resolve_directory,
        )
    }
}

/// An absolute VFS path retained for consumers awaiting typed backend migration.
///
/// The path is absolute but not necessarily lexically normalized: `.` and `..` are left for the
/// selected backend to interpret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPath(String);

impl ResolvedPath {
    /// Returns the absolute path.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the resolved path, returning the absolute path.
    pub fn into_string(self) -> String {
        self.0
    }
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Resolves a directory descriptor and path pair into an absolute VFS path.
///
/// This compatibility API retains the existing absolute-string result while anchoring through
/// [`AnchoredPath`]. New backend resolvers should consume [`AnchoredPath`] directly.
///
/// # Parameters
///
/// - `dirfd`: Directory descriptor used for relative paths.
/// - `path`: Raw path supplied by the caller.
///
/// # Returns
///
/// An absolute path without lexical normalization.
///
/// # Errors
///
/// Returns [`Fat32Error::NotFound`] if `path` is empty, [`Fat32Error::InvalidFd`] if a relative
/// path names an absent descriptor, or [`Fat32Error::NotADirectory`] if that descriptor is not a
/// directory. A relative `AT_FDCWD` path returns [`Fat32Error::InvalidArgument`] before VFS
/// initialization.
///
/// # Limitations
///
/// For hostfs directory descriptors, resolution uses the path stored at open time. If the directory
/// is renamed after being opened, subsequent operations using this descriptor resolve against the
/// stale path.
///
/// # References
///
/// - [POSIX openat()/`*at()` family — dirfd and `AT_FDCWD` semantics](https://pubs.opengroup.org/onlinepubs/9799919799/functions/openat.html)
pub fn vfs_resolve_path(dirfd: c_int, path: &str) -> Result<ResolvedPath, Fat32Error> {
    // TODO (#3101): Remove this absolute-string compatibility projection after all backends consume
    // AnchoredPath directly.
    let anchored: AnchoredPath = AnchoredPath::new(dirfd, String::from(path));
    Ok(ResolvedPath(anchored.into_absolute()?))
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Anchors a raw path using injected current-directory and descriptor lookups.
fn anchor_with<C, D>(
    anchored: AnchoredPath,
    current_directory: C,
    descriptor_directory: D,
) -> Result<String, Fat32Error>
where
    C: FnOnce() -> Result<String, Fat32Error>,
    D: FnOnce(c_int) -> Result<String, Fat32Error>,
{
    use ::sysapi::fcntl::atflags::AT_FDCWD;

    if anchored.path.is_empty() {
        return Err(Fat32Error::NotFound);
    }
    if anchored.path.starts_with('/') {
        return Ok(anchored.path);
    }

    let base: String = if anchored.dirfd == AT_FDCWD {
        current_directory()?
    } else {
        descriptor_directory(anchored.dirfd)?
    };
    Ok(join(&base, &anchored.path))
}

/// Returns the namespace path associated with a directory descriptor.
fn resolve_directory(dirfd: c_int) -> Result<String, Fat32Error> {
    let file: OpenFile = entry_arc(dirfd).map_err(|_| Fat32Error::InvalidFd)?;
    let guard = file.lock();
    let path: &str = match &guard.handle {
        VfsFileHandle::Directory(handle) => handle.path(),
        VfsFileHandle::HostFs(handle) if handle.is_dir() => {
            handle.path().ok_or(Fat32Error::InvalidFd)?
        },
        _ => return Err(Fat32Error::NotADirectory),
    };
    Ok(String::from(path))
}

/// Joins `path` onto an absolute `base` without lexical normalization.
fn join(base: &str, path: &str) -> String {
    if base.ends_with('/') {
        format!("{}{}", base, path)
    } else {
        format!("{}/{}", base, path)
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::sysapi::fcntl::atflags::AT_FDCWD;

    #[test]
    fn construction_preserves_raw_pair() {
        for (dirfd, path) in [
            (AT_FDCWD, ""),
            (17, "/absolute/../path"),
            (AT_FDCWD, "relative/path"),
            (42, "relative/./path"),
        ] {
            let anchored: AnchoredPath = AnchoredPath::new(dirfd, String::from(path));
            assert_eq!(anchored.dirfd(), dirfd);
            assert_eq!(anchored.path(), path);
        }
    }

    #[test]
    fn empty_path_is_rejected_when_consumed() {
        let result: Result<String, Fat32Error> = anchor_with(
            AnchoredPath::new(AT_FDCWD, String::new()),
            || Ok(String::from("/cwd")),
            |_| Ok(String::from("/descriptor")),
        );
        assert_eq!(result, Err(Fat32Error::NotFound));
    }

    #[test]
    fn absolute_path_ignores_directory_descriptor() {
        let result: String = anchor_with(
            AnchoredPath::new(-99, String::from("/absolute/../path")),
            || Err(Fat32Error::InvalidArgument),
            |_| Err(Fat32Error::InvalidFd),
        )
        .expect("absolute path should not inspect its directory descriptor");
        assert_eq!(result, "/absolute/../path");
    }

    #[test]
    fn relative_path_uses_current_directory() {
        let result: String = anchor_with(
            AnchoredPath::new(AT_FDCWD, String::from("relative/path")),
            || Ok(String::from("/cwd")),
            |_| Err(Fat32Error::InvalidFd),
        )
        .expect("AT_FDCWD path should use the current directory");
        assert_eq!(result, "/cwd/relative/path");
    }

    #[test]
    fn relative_path_uses_directory_descriptor() {
        let result: String = anchor_with(
            AnchoredPath::new(17, String::from("relative/path")),
            || Err(Fat32Error::InvalidArgument),
            |dirfd| {
                assert_eq!(dirfd, 17);
                Ok(String::from("/descriptor"))
            },
        )
        .expect("relative path should use its directory descriptor");
        assert_eq!(result, "/descriptor/relative/path");
    }

    #[test]
    fn anchor_errors_are_preserved() {
        let uninitialized: Result<String, Fat32Error> = anchor_with(
            AnchoredPath::new(AT_FDCWD, String::from("relative")),
            || Err(Fat32Error::InvalidArgument),
            |_| Ok(String::from("/descriptor")),
        );
        assert_eq!(uninitialized, Err(Fat32Error::InvalidArgument));

        for error in [Fat32Error::InvalidFd, Fat32Error::NotADirectory] {
            let result: Result<String, Fat32Error> = anchor_with(
                AnchoredPath::new(17, String::from("relative")),
                || Ok(String::from("/cwd")),
                |_| Err(error),
            );
            assert_eq!(result, Err(error));
        }
    }
}
