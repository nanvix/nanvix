// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! FAT32 filesystem backend for VFS FD operations.
//!
//! This module translates POSIX open flags into `fat32::OpenOptions` (via the
//! VFS high-level API) and produces [`VfsFileHandle`] variants for the FD
//! table. It also provides zero-copy direct-read handles for contiguous files.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    descriptor::{
        DirectReadHandle,
        DirectoryHandle,
        VfsFileHandle,
        VfsStat,
    },
    filesystem,
};
use ::alloc::string::String;
use ::fat32::Fat32Error;
use ::sysapi::{
    fcntl::{
        file_access_mode,
        file_creation_flags,
    },
    ffi::c_int,
};

//==================================================================================================
// Path Operations
//==================================================================================================

/// Returns `true` if the given path is handled by any VFS mount.
///
/// Checks both the path itself and its parent directory against
/// registered mount points.
///
/// # Parameters
///
/// - `path`: Absolute or relative path to check.
pub fn exists(cwd: &str, path: &str) -> bool {
    if filesystem::stat(cwd, path).is_ok() {
        return true;
    }
    if let Some(pos) = path.rfind('/') {
        let parent: &str = if pos == 0 { "/" } else { &path[..pos] };
        return filesystem::stat(cwd, parent).is_ok();
    }
    // Relative path with no directory separator — check whether the current
    // working directory itself lives inside a VFS mount.
    filesystem::stat(cwd, ".").is_ok()
}

/// Gets file metadata for the given path.
///
/// # Parameters
///
/// - `path`: Absolute path to query.
///
/// # Returns
///
/// [`VfsStat`] on success, or a [`Fat32Error`] on error.
pub fn stat(cwd: &str, path: &str) -> Result<VfsStat, Fat32Error> {
    let info: filesystem::Stat = filesystem::stat(cwd, path)?;
    Ok(VfsStat::new(info.size(), info.is_dir()))
}

//==================================================================================================
// File Operations
//==================================================================================================

/// Opens a file via the VFS and returns a [`VfsFileHandle`].
///
/// Translates POSIX `open()` flags into VFS `OpenOptions`. For read-only
/// opens of contiguous files, returns a zero-copy [`VfsFileHandle::DirectRead`]
/// handle.
///
/// # Parameters
///
/// - `path`: Absolute path to the file.
/// - `flags`: POSIX open flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, etc.).
///
/// # Returns
///
/// A [`VfsFileHandle`] on success, or a [`Fat32Error`] on error.
pub fn open(cwd: &str, path: &str, flags: c_int) -> Result<VfsFileHandle, Fat32Error> {
    // Handle O_DIRECTORY or paths that resolve to directories.
    // POSIX allows opening directories with O_RDONLY for fchdir()/getdents().
    if flags & file_creation_flags::O_DIRECTORY != 0 {
        let info: VfsStat = stat(cwd, path)?;
        if !info.is_dir() {
            return Err(Fat32Error::NotADirectory);
        }
        let normalized: String = filesystem::normalize(cwd, path)?;
        return Ok(VfsFileHandle::Directory(DirectoryHandle::new(normalized)));
    }

    // Auto-detect directories even without O_DIRECTORY flag.
    if let Ok(info) = stat(cwd, path) {
        if info.is_dir() {
            let normalized: String = filesystem::normalize(cwd, path)?;
            return Ok(VfsFileHandle::Directory(DirectoryHandle::new(normalized)));
        }
    }

    let access_mode: c_int = flags & file_access_mode::O_ACCMODE;
    let is_read_only: bool = access_mode == file_access_mode::O_RDONLY;

    // Try zero-copy direct read for read-only opens of contiguous files.
    let creation_flags: c_int =
        file_creation_flags::O_CREAT | file_creation_flags::O_TRUNC | file_creation_flags::O_EXCL;
    if is_read_only && (flags & creation_flags) == 0 {
        if let Some((data_ptr, size)) = filesystem::file_raw_region(cwd, path) {
            return Ok(VfsFileHandle::DirectRead(DirectReadHandle::new(data_ptr, size)));
        }
    }

    // Fall back to standard VFS open.
    let read: bool = access_mode != file_access_mode::O_WRONLY;
    let write: bool =
        access_mode == file_access_mode::O_WRONLY || access_mode == file_access_mode::O_RDWR;
    let create_requested: bool = flags & file_creation_flags::O_CREAT != 0;
    let create_new: bool = create_requested && flags & file_creation_flags::O_EXCL != 0;
    let create: bool = create_requested && !create_new;
    let truncate: bool = flags & file_creation_flags::O_TRUNC != 0;

    let file: filesystem::File =
        filesystem::open_with_options(cwd, path, read, write, create, create_new, truncate)?;
    Ok(VfsFileHandle::Fat32(file))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that `exists()` returns false for a path with no mounts.
    #[test]
    fn exists_no_mounts_returns_false() {
        // Without any VFS initialization, no path should exist.
        assert!(!exists("/", "/nonexistent"), "path should not exist without mounts");
    }

    /// Tests that `stat()` returns an error for a non-existent path.
    #[test]
    fn stat_nonexistent_returns_error() {
        let result: Result<VfsStat, Fat32Error> = stat("/", "/nonexistent");
        assert!(result.is_err(), "stat on non-existent path should fail");
    }
}
