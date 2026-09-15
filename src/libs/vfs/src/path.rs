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
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::c_int,
};

//==================================================================================================
// Structures
//==================================================================================================

/// How a raw path is anchored in the VFS namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathAnchor {
    /// An absolute path, along with the ignored descriptor supplied by the caller.
    Absolute(c_int),
    /// A relative path anchored at the current working directory.
    CurrentDirectory,
    /// A relative path anchored at a directory descriptor.
    Directory(c_int),
}

/// A raw directory descriptor and path pair supplied at a system-call boundary.
///
/// Construction validates backend-independent string constraints while preserving raw provenance.
/// A backend resolver later joins and normalizes the path according to its policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredPath {
    /// Typed anchoring mode selected from the raw descriptor and path.
    anchor: PathAnchor,
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
    /// A checked path value that preserves the supplied pair without joining or normalization.
    ///
    /// # Errors
    ///
    /// Returns [`Fat32Error::NotFound`] if `path` is empty or [`Fat32Error::InvalidPath`] if it
    /// contains an embedded null byte.
    pub fn new(dirfd: c_int, path: String) -> Result<Self, Fat32Error> {
        Self::validate_raw(&path)?;
        let anchor: PathAnchor = if path.starts_with('/') {
            PathAnchor::Absolute(dirfd)
        } else if dirfd == AT_FDCWD {
            PathAnchor::CurrentDirectory
        } else {
            PathAnchor::Directory(dirfd)
        };

        Ok(Self { anchor, path })
    }

    /// Returns the directory descriptor supplied by the caller.
    pub fn dirfd(&self) -> c_int {
        match self.anchor {
            PathAnchor::Absolute(dirfd) | PathAnchor::Directory(dirfd) => dirfd,
            PathAnchor::CurrentDirectory => AT_FDCWD,
        }
    }

    /// Returns the raw path supplied by the caller.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Consumes this value and anchors its path in the VFS namespace.
    ///
    /// This does not inspect VFS initialization because callers may already hold `VFS_STATE`.
    pub(crate) fn into_absolute(self) -> Result<String, Fat32Error> {
        match self.anchor {
            PathAnchor::Absolute(_) => Ok(self.path),
            PathAnchor::CurrentDirectory => Ok(join(&current_cwd(), &self.path)),
            PathAnchor::Directory(dirfd) => Ok(join(&resolve_directory(dirfd)?, &self.path)),
        }
    }

    /// Returns whether this path is relative to the current working directory.
    fn uses_current_directory(&self) -> bool {
        matches!(self.anchor, PathAnchor::CurrentDirectory)
    }

    /// Validates constraints that apply before backend-specific path processing.
    /// Empty and null-containing paths are invalid; component spelling is backend-specific.
    pub(crate) fn validate_raw(path: &str) -> Result<(), Fat32Error> {
        if path.is_empty() {
            return Err(Fat32Error::NotFound);
        }
        if path.contains('\0') {
            return Err(Fat32Error::InvalidPath);
        }
        Ok(())
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
/// Returns [`Fat32Error::NotFound`] if `path` is empty, [`Fat32Error::InvalidPath`] if it contains
/// an embedded null byte, [`Fat32Error::InvalidFd`] if a relative path names an absent descriptor,
/// or [`Fat32Error::NotADirectory`] if that descriptor is not a directory. A relative `AT_FDCWD`
/// path returns [`Fat32Error::InvalidArgument`] before VFS initialization.
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
    let anchored: AnchoredPath = AnchoredPath::new(dirfd, String::from(path))?;
    if anchored.uses_current_directory() && !crate::state::is_initialized() {
        return Err(Fat32Error::InvalidArgument);
    }
    Ok(ResolvedPath(anchored.into_absolute()?))
}

//==================================================================================================
// Private Functions
//==================================================================================================

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

    #[test]
    fn construction_selects_explicit_anchor_state() {
        for (dirfd, path, expected) in [
            (17, "/absolute/../path", PathAnchor::Absolute(17)),
            (AT_FDCWD, "relative/path", PathAnchor::CurrentDirectory),
            (42, "relative/./path", PathAnchor::Directory(42)),
        ] {
            let anchored: AnchoredPath = AnchoredPath::new(dirfd, String::from(path))
                .expect("valid raw path should be accepted");
            assert_eq!(anchored.anchor, expected);
            assert_eq!(anchored.dirfd(), dirfd);
            assert_eq!(anchored.path(), path);
        }
    }

    #[test]
    fn construction_rejects_invalid_raw_paths() {
        assert_eq!(AnchoredPath::new(AT_FDCWD, String::new()), Err(Fat32Error::NotFound),);
        for (dirfd, path) in [
            (-99, "/invalid\0/../absolute"),
            (AT_FDCWD, "invalid\0/../relative"),
        ] {
            assert_eq!(AnchoredPath::new(dirfd, String::from(path)), Err(Fat32Error::InvalidPath),);
        }
    }

    #[test]
    fn absolute_path_ignores_directory_descriptor() {
        let path: AnchoredPath = AnchoredPath::new(-99, String::from("/absolute/../path"))
            .expect("valid raw path should be accepted");
        let result: String = path
            .into_absolute()
            .expect("absolute path should not inspect its directory descriptor");
        assert_eq!(result, "/absolute/../path");
    }

    #[test]
    fn missing_directory_descriptor_is_rejected() {
        let path: AnchoredPath = AnchoredPath::new(c_int::MAX, String::from("relative/path"))
            .expect("valid raw path should be accepted");
        assert_eq!(path.into_absolute(), Err(Fat32Error::InvalidFd));
    }
}
