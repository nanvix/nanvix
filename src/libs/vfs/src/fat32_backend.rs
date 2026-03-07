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

use crate::fd::{
    DirectReadHandle,
    DirectoryHandle,
    VfsFileHandle,
    VfsStat,
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
pub fn exists(path: &str) -> bool {
    if crate::stat(path).is_ok() {
        return true;
    }
    if let Some(pos) = path.rfind('/') {
        let parent: &str = if pos == 0 { "/" } else { &path[..pos] };
        return crate::stat(parent).is_ok();
    }
    // Relative path with no directory separator — check whether the current
    // working directory itself lives inside a VFS mount.
    crate::stat(".").is_ok()
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
pub fn stat(path: &str) -> Result<VfsStat, Fat32Error> {
    let info: crate::Stat = crate::stat(path)?;
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
pub fn open(path: &str, flags: c_int) -> Result<VfsFileHandle, Fat32Error> {
    // Handle O_DIRECTORY or paths that resolve to directories.
    // POSIX allows opening directories with O_RDONLY for fchdir()/getdents().
    if flags & file_creation_flags::O_DIRECTORY != 0 {
        let info: VfsStat = stat(path)?;
        if !info.is_dir() {
            return Err(Fat32Error::NotADirectory);
        }
        let normalized: String = crate::normalize(path)?;
        return Ok(VfsFileHandle::Directory(DirectoryHandle::new(normalized)));
    }

    // Auto-detect directories even without O_DIRECTORY flag.
    if let Ok(info) = stat(path) {
        if info.is_dir() {
            let normalized: String = crate::normalize(path)?;
            return Ok(VfsFileHandle::Directory(DirectoryHandle::new(normalized)));
        }
    }

    let access_mode: c_int = flags & file_access_mode::O_ACCMODE;
    let is_read_only: bool = access_mode == file_access_mode::O_RDONLY;

    // Try zero-copy direct read for read-only opens of contiguous files.
    let creation_flags: c_int =
        file_creation_flags::O_CREAT | file_creation_flags::O_TRUNC | file_creation_flags::O_EXCL;
    if is_read_only && (flags & creation_flags) == 0 {
        if let Some((data_ptr, size)) = crate::file_raw_region(path) {
            return Ok(VfsFileHandle::DirectRead(DirectReadHandle::new(data_ptr, size)));
        }
    }

    // Fall back to standard VFS open.
    let mut opts: crate::OpenOptions = crate::OpenOptions::new();

    if access_mode == file_access_mode::O_WRONLY {
        opts = opts.write(true);
    } else if access_mode == file_access_mode::O_RDWR {
        opts = opts.read(true).write(true);
    } else {
        opts = opts.read(true);
    }

    if flags & file_creation_flags::O_CREAT != 0 {
        if flags & file_creation_flags::O_EXCL != 0 {
            opts = opts.create_new(true);
        } else {
            opts = opts.create(true);
        }
    }

    if flags & file_creation_flags::O_TRUNC != 0 {
        opts = opts.truncate(true);
    }

    let file: crate::File = opts.open(path)?;
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
        assert!(!exists("/nonexistent"), "path should not exist without mounts");
    }

    /// Tests that `stat()` returns an error for a non-existent path.
    #[test]
    fn stat_nonexistent_returns_error() {
        let result: Result<VfsStat, Fat32Error> = stat("/nonexistent");
        assert!(result.is_err(), "stat on non-existent path should fail");
    }
}
