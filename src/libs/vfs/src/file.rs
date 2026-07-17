// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Unified file handle and POSIX-like filesystem operations.
//!
//! This module provides:
//! - [`File`]: A unified file handle for FAT filesystem files.
//! - [`OpenOptions`]: A builder for opening files with specific access modes.
//! - Free functions for filesystem operations: [`open()`], [`stat()`],
//!   [`mkdir()`], [`rmdir()`], [`unlink()`], [`rename()`], [`read_dir()`],
//!   [`chdir()`], [`cwd()`].

//==================================================================================================
// Modules
//==================================================================================================

mod dir_entry;
mod handle;
mod open_options;
mod stat;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use self::{
    dir_entry::DirEntry,
    handle::File,
    open_options::OpenOptions,
    stat::Stat,
};

//==================================================================================================
// Imports
//==================================================================================================

use crate::state;
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::fat32::{
    Fat32Error,
    FatFile,
};

//==================================================================================================
// Public API Functions
//==================================================================================================

/// Opens a file by path for reading.
///
/// Routes through the VFS to the appropriate FAT backend.
///
/// # Parameters
///
/// - `path`: The path to the file to open.
///
/// # Returns
///
/// A new [`File`] handle opened for reading.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::NotFound`] if the path doesn't exist.
pub fn open(path: &str) -> Result<File, Fat32Error> {
    open_with_options(path, true, false, false, false, false)
}

/// Returns a pointer and size for zero-copy access to a file's data in
/// the in-memory FAT image.
///
/// If the file is stored contiguously, returns `Some((data_ptr, file_size))`
/// where `data_ptr` points directly into the FAT image buffer. Returns
/// `None` if the file is empty, not found, fragmented, or no mount handles
/// the path.
///
/// # Parameters
///
/// - `path`: The path to the file.
pub fn file_raw_region(path: &str) -> Option<(*const u8, usize)> {
    let (mount_idx, relative_path): (usize, String) = resolve_path(path).ok()?;
    state::with_vfs(|vfs| {
        let mount: &crate::mount::Mount = vfs.get_mount(mount_idx).ok_or(Fat32Error::NotFound)?;
        mount
            .fat()
            .file_raw_region(&relative_path)
            .ok_or(Fat32Error::NotFound)
    })
    .ok()
}

/// Gets file metadata without opening the file.
///
/// # Parameters
///
/// - `path`: The path to query.
///
/// # Returns
///
/// File metadata.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::NotFound`] if the path doesn't exist.
pub fn stat(path: &str) -> Result<Stat, Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // Handle root of mount specially.
    if relative_path.is_empty() {
        return Ok(Stat::new(0, true));
    }

    state::with_vfs(|vfs| {
        let mount = vfs.get_mount(mount_idx).ok_or(Fat32Error::NotFound)?;
        let fat_stat = mount.fat().stat(&relative_path)?;
        Ok(Stat::new(fat_stat.size, fat_stat.is_dir))
    })
}

/// Creates a directory.
///
/// # Parameters
///
/// - `path`: The path to the directory to create.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::ReadOnly`] if the mount is read-only.
/// - [`Fat32Error::AlreadyExists`] if directory already exists.
/// - [`Fat32Error::NotFound`] if parent directory doesn't exist.
pub fn mkdir(path: &str) -> Result<(), Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // Root of a mount always exists — return AlreadyExists (mirrors stat()).
    if relative_path.is_empty() {
        return Err(Fat32Error::NotFound);
    }

    check_writable(mount_idx)?;

    state::with_vfs_mut(|vfs| {
        let mount = vfs.get_mount_mut(mount_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat_mut().mkdir(&relative_path)
    })
}

/// Removes an empty directory.
///
/// # Parameters
///
/// - `path`: The path to the directory to remove.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::ReadOnly`] if the mount is read-only.
/// - [`Fat32Error::NotFound`] if directory doesn't exist.
/// - [`Fat32Error::NotEmpty`] if directory is not empty.
/// - [`Fat32Error::NotADirectory`] if path is a file.
pub fn rmdir(path: &str) -> Result<(), Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // Cannot remove the root of a mount.
    if relative_path.is_empty() {
        return Err(Fat32Error::NotFound);
    }

    check_writable(mount_idx)?;

    state::with_vfs_mut(|vfs| {
        let mount = vfs.get_mount_mut(mount_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat_mut().rmdir(&relative_path)
    })
}

/// Deletes a file.
///
/// # Parameters
///
/// - `path`: The path to the file to delete.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::ReadOnly`] if the mount is read-only.
/// - [`Fat32Error::NotFound`] if file doesn't exist.
/// - [`Fat32Error::NotAFile`] if path is a directory.
pub fn unlink(path: &str) -> Result<(), Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // Root of a mount is a directory, not a file.
    if relative_path.is_empty() {
        return Err(Fat32Error::NotFound);
    }

    check_writable(mount_idx)?;

    state::with_vfs_mut(|vfs| {
        let mount = vfs.get_mount_mut(mount_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat_mut().unlink(&relative_path)
    })
}

/// Lists the contents of a directory.
///
/// # Parameters
///
/// - `path`: The path to the directory to list.
///
/// # Returns
///
/// A vector of directory entries (direct children only, not recursive).
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::NotFound`] if the path doesn't exist.
/// - [`Fat32Error::NotADirectory`] if the path is a file.
pub fn read_dir(path: &str) -> Result<Vec<DirEntry>, Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    state::with_vfs(|vfs| {
        let mount = vfs.get_mount(mount_idx).ok_or(Fat32Error::NotFound)?;

        let fat_path: &str = if relative_path.is_empty() {
            "."
        } else {
            &relative_path
        };

        let fat_entries = mount.fat().read_dir(fat_path)?;

        let entries: Vec<DirEntry> = fat_entries
            .into_iter()
            .map(|e| DirEntry::new(e.name, e.is_dir, e.size))
            .collect();

        Ok(entries)
    })
}

/// Renames a file or directory.
///
/// Both paths must be on the same mount.
///
/// # Parameters
///
/// - `old_path`: Current path of the file or directory.
/// - `new_path`: New path for the file or directory.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::NotFound`] if `old_path` doesn't exist.
/// - [`Fat32Error::AlreadyExists`] if `new_path` already exists.
/// - [`Fat32Error::InvalidPath`] if paths are on different mounts.
pub fn rename(old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
    let (old_idx, old_rel) = resolve_path(old_path)?;
    let (new_idx, new_rel) = resolve_path(new_path)?;

    // Cannot rename mount roots.
    if old_rel.is_empty() || new_rel.is_empty() {
        return Err(Fat32Error::NotFound);
    }

    // Both must be on the same mount.
    if old_idx != new_idx {
        return Err(Fat32Error::InvalidPath);
    }

    check_writable(old_idx)?;

    state::with_vfs(|vfs| {
        let mount = vfs.get_mount(old_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat().rename(&old_rel, &new_rel)
    })
}

/// Gets the current working directory.
///
/// # Returns
///
/// The absolute path of the current working directory.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
pub fn cwd() -> Result<String, Fat32Error> {
    // The working directory now lives in the per-process registry rather than on the `Vfs`, but the
    // documented contract still requires the filesystem to be initialized first.
    if !state::is_initialized() {
        return Err(Fat32Error::NotInitialized);
    }
    Ok(crate::fd::current_cwd())
}

/// Changes the current working directory.
///
/// # Parameters
///
/// - `path`: The new working directory path (absolute or relative).
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::InvalidPath`] if the path is malformed.
/// - [`Fat32Error::NotFound`] if no mount handles this path.
pub fn chdir(path: &str) -> Result<(), Fat32Error> {
    // The working directory lives solely in the per-process registry. Normalize against the
    // process's current working directory and validate that some mount handles the target (root is
    // always valid), then persist it so it survives context switches and is inherited by forked
    // children.
    let cwd: String = crate::fd::current_cwd();
    let normalized: String = state::with_vfs_mut(|vfs| {
        let normalized: String = vfs.normalize_path(path, &cwd)?;
        if !normalized.is_empty() && normalized != "/" {
            let _ = vfs.resolve(&normalized, &cwd)?;
        }
        Ok(normalized)
    })?;
    crate::fd::set_current_cwd(normalized);
    Ok(())
}

/// Normalizes a path to an absolute path using the current working directory.
///
/// # Parameters
///
/// - `path`: The path to normalize.
///
/// # Returns
///
/// The normalized absolute path.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`Fat32Error::InvalidPath`] if the path is malformed.
pub fn normalize(path: &str) -> Result<String, Fat32Error> {
    let cwd: String = crate::fd::current_cwd();
    state::with_vfs(|vfs| vfs.normalize_path(path, &cwd))
}

//==================================================================================================
// Internal Functions
//==================================================================================================

/// Resolves a path through the VFS to determine which mount handles it.
///
/// # Parameters
///
/// - `path`: The path to resolve.
///
/// # Returns
///
/// A tuple of `(mount_index, relative_path)`.
fn resolve_path(path: &str) -> Result<(usize, String), Fat32Error> {
    let cwd: String = crate::fd::current_cwd();
    state::with_vfs_mut(|vfs| vfs.resolve(path, &cwd))
}

/// Returns [`Fat32Error::ReadOnly`] if the mount at `mount_idx` is read-only.
/// Used to gate mutating operations.
fn check_writable(mount_idx: usize) -> Result<(), Fat32Error> {
    let readonly: bool = state::with_vfs(|vfs| {
        let mount = vfs.get_mount(mount_idx).ok_or(Fat32Error::NotFound)?;
        Ok(mount.readonly())
    })?;
    if readonly {
        return Err(Fat32Error::ReadOnly);
    }
    Ok(())
}

/// Opens a file with specific options.
///
/// # Parameters
///
/// - `path`: The path to the file.
/// - `read`: Open for reading.
/// - `write`: Open for writing.
/// - `create`: Create if doesn't exist.
/// - `create_new`: Fail if already exists (O_EXCL).
/// - `truncate`: Truncate to zero length.
fn open_with_options(
    path: &str,
    read: bool,
    write: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
) -> Result<File, Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // Root of a mount is a directory, not a file — cannot be opened as a file.
    if relative_path.is_empty() {
        return Err(Fat32Error::NotFound);
    }

    // Reject write/create/truncate on read-only mounts.
    // NOTE: This gate is also what keeps the negative cache consistent —
    // negative entries are only populated for read-only mounts, so any
    // O_CREAT that could create a file is blocked here before it reaches
    // the FAT layer, preventing stale negative-cache entries.
    if write || create || create_new || truncate {
        check_writable(mount_idx)?;
    }

    // Open the file under a single VFS lock scope, resolving both the
    // mount path and file handle together. This avoids aliased &/&mut
    // references that the previous implementation created.
    let (fat_file, mount_path) = state::with_vfs_mut(|vfs| {
        let mount = vfs.get_mount_mut(mount_idx).ok_or(Fat32Error::NotFound)?;
        let mount_path: String = String::from(mount.path());

        let fat_file = if create_new {
            mount.fat_mut().create_new(&relative_path, read, write)?
        } else {
            mount
                .fat_mut()
                .open(&relative_path, read, write, create, truncate)?
        };

        // SAFETY: The FatFile borrows from the FAT filesystem stored in the
        // global VFS static. This lifetime extension is safe because:
        // 1. The VFS is stored in a global static and is never dropped.
        // 2. Mounts with open files cannot be removed — `state::unmount()`
        //    checks `has_open_files()` and returns `FileLocked` if any
        //    `File` handles (which call `increment_open_count` here and
        //    `decrement_open_count` on `Drop`) are still alive.
        // 3. The Mutex ensures exclusive access during file handle creation.
        //
        // IMPORTANT: Any change to `state::unmount()` or the open-file
        // counting must preserve this invariant; otherwise this is unsound.
        // See also the TOCTOU note in `state::unmount()`.
        let fat_file: FatFile<'static> = unsafe { core::mem::transmute(fat_file) };

        Ok((fat_file, mount_path))
    })?;

    // Track the open file (outside VFS lock to avoid nested locking).
    state::increment_open_count(&mount_path);

    Ok(File::new(fat_file, mount_path))
}
