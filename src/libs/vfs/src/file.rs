// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Process-aware filesystem facade.
//!
//! This module preserves the public VFS file API while delegating storage operations to the
//! process-independent [`crate::filesystem`] core.

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use crate::{
    filesystem::{
        DirEntry,
        File,
        Stat,
    },
    open_options::OpenOptions,
};

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fd,
    filesystem,
    state,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::fat32::Fat32Error;

//==================================================================================================
// Public API Functions
//==================================================================================================

/// Opens a file by path for reading.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized, the path is invalid, or the file does
/// not exist.
pub fn open(path: &str) -> Result<File, Fat32Error> {
    filesystem::open(&fd::current_cwd(), path)
}

/// Returns a pointer and size for zero-copy access to a file's data.
#[must_use]
pub fn file_raw_region(path: &str) -> Option<(*const u8, usize)> {
    filesystem::file_raw_region(&fd::current_cwd(), path)
}

/// Gets file metadata without opening the file.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized or the path does not exist.
pub fn stat(path: &str) -> Result<Stat, Fat32Error> {
    filesystem::stat(&fd::current_cwd(), path)
}

/// Creates a directory.
///
/// # Errors
///
/// Returns an error when the path is invalid, already exists, or belongs to a read-only mount.
pub fn mkdir(path: &str) -> Result<(), Fat32Error> {
    filesystem::mkdir(&fd::current_cwd(), path)
}

/// Removes an empty directory.
///
/// # Errors
///
/// Returns an error when the path is invalid, nonempty, or belongs to a read-only mount.
pub fn rmdir(path: &str) -> Result<(), Fat32Error> {
    filesystem::rmdir(&fd::current_cwd(), path)
}

/// Deletes a file.
///
/// # Errors
///
/// Returns an error when the path is invalid, names a directory, or belongs to a read-only mount.
pub fn unlink(path: &str) -> Result<(), Fat32Error> {
    filesystem::unlink(&fd::current_cwd(), path)
}

/// Lists the contents of a directory.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized or the path is not a directory.
pub fn read_dir(path: &str) -> Result<Vec<DirEntry>, Fat32Error> {
    filesystem::read_dir(&fd::current_cwd(), path)
}

/// Renames a file or directory.
///
/// # Errors
///
/// Returns an error when either path is invalid, the paths resolve to different mounts, or the
/// mount is read-only.
pub fn rename(old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
    filesystem::rename(&fd::current_cwd(), old_path, new_path)
}

/// Gets the current working directory.
///
/// # Errors
///
/// Returns [`Fat32Error::NotInitialized`] if the filesystem is not initialized.
pub fn cwd() -> Result<String, Fat32Error> {
    if !state::is_initialized() {
        return Err(Fat32Error::NotInitialized);
    }
    Ok(fd::current_cwd())
}

/// Changes the current working directory.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized or the path cannot be resolved.
pub fn chdir(path: &str) -> Result<(), Fat32Error> {
    let cwd: String = fd::current_cwd();
    let normalized: String = filesystem::change_directory(&cwd, path)?;
    fd::set_current_cwd(normalized);
    Ok(())
}

/// Normalizes a path to an absolute path using the current working directory.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized or the path is malformed.
pub fn normalize(path: &str) -> Result<String, Fat32Error> {
    filesystem::normalize(&fd::current_cwd(), path)
}

//==================================================================================================
// Internal Functions
//==================================================================================================

/// Opens a file with specific access and creation options.
pub(crate) fn open_with_options(
    path: &str,
    read: bool,
    write: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
) -> Result<File, Fat32Error> {
    filesystem::open_with_options(
        &fd::current_cwd(),
        path,
        read,
        write,
        create,
        create_new,
        truncate,
    )
}
