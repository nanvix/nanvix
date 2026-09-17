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
    filesystem,
    path::AnchoredPath,
    process,
    state,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::fat32::Fat32Error;
use ::sysapi::fcntl::atflags::AT_FDCWD;

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
    filesystem::open_anchored(anchored(path)?)
}

/// Returns a pointer and size for zero-copy access to a file's data.
#[must_use]
pub fn file_raw_region(path: &str) -> Option<(*const u8, usize)> {
    let path = filesystem::VfsResolvedPath::new(anchored(path).ok()?).ok()?;
    filesystem::file_raw_region(&path)
}

/// Gets file metadata without opening the file.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized or the path does not exist.
pub fn stat(path: &str) -> Result<Stat, Fat32Error> {
    let path = filesystem::VfsResolvedPath::new(anchored(path)?)?;
    filesystem::stat(&path)
}

/// Creates a directory.
///
/// # Errors
///
/// Returns an error when the path is invalid, already exists, or belongs to a read-only mount.
pub fn mkdir(path: &str) -> Result<(), Fat32Error> {
    filesystem::mkdir(anchored(path)?)
}

/// Removes an empty directory.
///
/// # Errors
///
/// Returns an error when the path is invalid, nonempty, or belongs to a read-only mount.
pub fn rmdir(path: &str) -> Result<(), Fat32Error> {
    filesystem::rmdir(anchored(path)?)
}

/// Deletes a file.
///
/// # Errors
///
/// Returns an error when the path is invalid, names a directory, or belongs to a read-only mount.
pub fn unlink(path: &str) -> Result<(), Fat32Error> {
    filesystem::unlink(anchored(path)?)
}

/// Lists the contents of a directory.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized or the path is not a directory.
pub fn read_dir(path: &str) -> Result<Vec<DirEntry>, Fat32Error> {
    filesystem::read_dir(anchored(path)?)
}

/// Renames a file or directory.
///
/// # Errors
///
/// Returns an error when either path is invalid, the paths resolve to different mounts, or the
/// mount is read-only.
pub fn rename(old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
    filesystem::rename(anchored(old_path)?, anchored(new_path)?)
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
    Ok(process::current_cwd())
}

/// Changes the current working directory.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized or the path cannot be resolved.
pub fn chdir(path: &str) -> Result<(), Fat32Error> {
    let normalized = filesystem::change_directory(anchored(path)?)?;
    process::set_current_cwd(normalized);
    Ok(())
}

/// Normalizes a path to an absolute path using the current working directory.
///
/// # Errors
///
/// Returns an error when the filesystem is not initialized or the path is malformed.
pub fn normalize(path: &str) -> Result<String, Fat32Error> {
    filesystem::normalize(anchored(path)?)
}

//==================================================================================================
// Internal Functions
//==================================================================================================

/// Records a path relative to the process working directory.
fn anchored(path: &str) -> Result<AnchoredPath, Fat32Error> {
    AnchoredPath::new(AT_FDCWD, String::from(path))
}

/// Opens a file with specific access and creation options.
pub(crate) fn open_with_options(
    path: &str,
    read: bool,
    write: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
) -> Result<File, Fat32Error> {
    let path = filesystem::VfsResolvedPath::new(anchored(path)?)?;
    filesystem::open(&path, read, write, create, create_new, truncate)
}
