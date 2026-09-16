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
        AccessMode,
        DirectReadHandle,
        DirectoryHandle,
        NullHandle,
        VfsFileHandle,
    },
    devfs::DevicePath,
    filesystem,
    path::AnchoredPath,
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

/// Returns whether an anchored path or its parent exists in the local VFS.
pub fn exists(path: AnchoredPath) -> bool {
    if stat(path.clone()).is_ok() {
        return true;
    }

    let parent: &str = match path.path().rfind('/') {
        Some(0) => "/",
        Some(pos) => &path.path()[..pos],
        None => ".",
    };
    AnchoredPath::new(path.dirfd(), String::from(parent)).is_ok_and(|path| stat(path).is_ok())
}

/// Gets metadata for an anchored path.
fn stat(path: AnchoredPath) -> Result<filesystem::Stat, Fat32Error> {
    let path = filesystem::VfsResolvedPath::new(path)?;
    filesystem::stat(&path)
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
///
/// # References
///
/// - [POSIX open()](https://pubs.opengroup.org/onlinepubs/9799919799/functions/open.html)
/// - [POSIX pathname resolution (trailing slash rule)](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html)
pub fn open(path: &filesystem::VfsResolvedPath, flags: c_int) -> Result<VfsFileHandle, Fat32Error> {
    // POSIX allows opening directories with O_RDONLY for fchdir()/getdents().
    if flags & file_creation_flags::O_DIRECTORY != 0 {
        let info = filesystem::stat(path)?;
        if !info.is_dir() {
            return Err(Fat32Error::NotADirectory);
        }
        return directory_handle(path);
    }

    // A trailing slash forces directory semantics.
    if path.has_trailing_separator() {
        let info = filesystem::stat(path)?;
        if !info.is_dir() {
            return Err(Fat32Error::NotADirectory);
        }
        return directory_handle(path);
    }

    if filesystem::stat(path).is_ok_and(|info| info.is_dir()) {
        return directory_handle(path);
    }

    let access_mode: c_int = flags & file_access_mode::O_ACCMODE;
    let is_read_only: bool = access_mode == file_access_mode::O_RDONLY;
    match path.device() {
        Some(DevicePath::Null) => {
            if flags & file_access_mode::O_EXEC != 0 {
                return Err(Fat32Error::PermissionDenied);
            }
            if flags & file_creation_flags::O_CREAT != 0 && flags & file_creation_flags::O_EXCL != 0
            {
                return Err(Fat32Error::AlreadyExists);
            }
            let access_mode: AccessMode = match access_mode {
                file_access_mode::O_RDONLY => AccessMode::ReadOnly,
                file_access_mode::O_WRONLY => AccessMode::WriteOnly,
                file_access_mode::O_RDWR => AccessMode::ReadWrite,
                _ => return Err(Fat32Error::InvalidArgument),
            };
            return Ok(VfsFileHandle::Null(NullHandle::new(access_mode)));
        },
        Some(DevicePath::Missing) => {
            if flags & file_creation_flags::O_CREAT != 0 {
                return Err(Fat32Error::PermissionDenied);
            }
            return Err(Fat32Error::NotFound);
        },
        Some(DevicePath::Directory) => return Err(Fat32Error::NotAFile),
        Some(DevicePath::Tty | DevicePath::Console) | None => {},
    }

    let creation_flags: c_int =
        file_creation_flags::O_CREAT | file_creation_flags::O_TRUNC | file_creation_flags::O_EXCL;
    if is_read_only && (flags & creation_flags) == 0 {
        if let Some((data_ptr, size)) = filesystem::file_raw_region(path) {
            return Ok(VfsFileHandle::DirectRead(DirectReadHandle::new(data_ptr, size)));
        }
    }

    let read: bool = access_mode != file_access_mode::O_WRONLY;
    let write: bool =
        access_mode == file_access_mode::O_WRONLY || access_mode == file_access_mode::O_RDWR;
    let create_requested: bool = flags & file_creation_flags::O_CREAT != 0;
    let create_new: bool = create_requested && flags & file_creation_flags::O_EXCL != 0;
    let create: bool = create_requested && !create_new;
    let truncate: bool = flags & file_creation_flags::O_TRUNC != 0;
    let file = filesystem::open(path, read, write, create, create_new, truncate)?;
    Ok(VfsFileHandle::Fat32(file))
}

/// Creates a directory handle from an already-resolved path.
fn directory_handle(path: &filesystem::VfsResolvedPath) -> Result<VfsFileHandle, Fat32Error> {
    let normalized = String::from(path.normalized()?);
    Ok(VfsFileHandle::Directory(DirectoryHandle::new(normalized)))
}
