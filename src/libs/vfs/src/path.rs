// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Resolved VFS paths.
//!
//! [`ResolvedPath`] records that a `dirfd` + `path` pair has been anchored into
//! an absolute path. The inner value is private to this module and
//! [`vfs_resolve_path`] is the only way to produce one, so holding a
//! `ResolvedPath` is proof that resolution actually happened.

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

/// An absolute VFS path produced by [`vfs_resolve_path`].
///
/// Carries the proof that a `dirfd` + `path` pair has already been anchored, so
/// callees need not re-check it. The path is absolute but not necessarily
/// lexically normalized: `.`/`..` are left for whoever owns the path to
/// interpret, since hostfs resolves them against the real filesystem.
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

/// Resolves a `dirfd` + `path` pair into an absolute VFS path.
///
/// If `path` is absolute, it is returned as-is (dirfd is ignored per POSIX).
/// If `dirfd` is `AT_FDCWD`, the path is resolved against the VFS current
/// working directory. If `dirfd` is a directory descriptor, the path is resolved
/// relative to that directory's path.
///
/// Returns [`Fat32Error::NotFound`] if `path` is empty (POSIX `ENOENT`).
/// Returns [`Fat32Error::InvalidFd`] if `dirfd` is not an open descriptor of the current process
/// (POSIX `EBADF`), or [`Fat32Error::NotADirectory`] if it is an open descriptor that does not
/// refer to a directory (POSIX `ENOTDIR`).
///
/// # Limitations
///
/// For hostfs directory fds, resolution uses the path stored at open time.
/// If the directory is renamed after being opened, subsequent `*at()` calls
/// using this dirfd will resolve against the stale path. A future protocol
/// extension could support `*at()` operations relative to a remote directory
/// FD on the host side to provide stable POSIX-like dirfd semantics.
///
/// # References
///
/// - [POSIX openat()/`*at()` family — dirfd and `AT_FDCWD` semantics](https://pubs.opengroup.org/onlinepubs/9799919799/functions/openat.html)
pub fn vfs_resolve_path(dirfd: c_int, path: &str) -> Result<ResolvedPath, Fat32Error> {
    use ::sysapi::fcntl::atflags::AT_FDCWD;

    // An empty path names nothing. Anchoring one would silently yield the base
    // directory, so reject it here with ENOENT rather than resolve to the cwd.
    if path.is_empty() {
        return Err(Fat32Error::NotFound);
    }

    // Absolute paths are always resolved directly (dirfd ignored per POSIX).
    if path.starts_with('/') {
        return Ok(ResolvedPath(String::from(path)));
    }

    // Relative path with AT_FDCWD: resolve against VFS cwd.
    if dirfd == AT_FDCWD {
        if !crate::state::is_initialized() {
            return Err(Fat32Error::InvalidArgument);
        }
        let cwd: String = current_cwd();
        return Ok(ResolvedPath(join(&cwd, path)));
    }

    // Relative path with a directory descriptor: resolve against that directory. Validity is the
    // slot's handle type, not the descriptor number — an absent descriptor is `EBADF` and a
    // non-directory descriptor is `ENOTDIR`, per POSIX.
    let file: OpenFile = entry_arc(dirfd).map_err(|_| Fat32Error::InvalidFd)?;
    let guard = file.lock();
    let dir_path: &str = match &guard.handle {
        VfsFileHandle::Directory(dh) => dh.path(),
        VfsFileHandle::HostFs(hh) if hh.is_dir() => hh.path().ok_or(Fat32Error::InvalidFd)?,
        _ => return Err(Fat32Error::NotADirectory), // fd is not a directory
    };

    Ok(ResolvedPath(join(dir_path, path)))
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Joins `path` onto the absolute `base`.
fn join(base: &str, path: &str) -> String {
    if base.ends_with('/') {
        format!("{}{}", base, path)
    } else {
        format!("{}/{}", base, path)
    }
}
