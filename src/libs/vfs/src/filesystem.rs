// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Process-independent filesystem operations.
//!
//! This module provides:
//! - [`File`]: A unified file handle for FAT filesystem files.
//! - Free functions that resolve paths against an explicit working directory.

//==================================================================================================
// Modules
//==================================================================================================

mod dir_entry;
mod handle;
mod stat;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use self::{
    dir_entry::DirEntry,
    handle::File,
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
    FAT_EPOCH_SECS,
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
pub(crate) fn open(cwd: &str, path: &str) -> Result<File, Fat32Error> {
    open_with_options(cwd, path, true, false, false, false, false)
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
pub(crate) fn file_raw_region(cwd: &str, path: &str) -> Option<(*const u8, usize)> {
    let (mount_idx, relative_path): (usize, String) = resolve_path(cwd, path).ok()?;
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
pub(crate) fn stat(cwd: &str, path: &str) -> Result<Stat, Fat32Error> {
    let requires_dir: bool = path.ends_with('/');
    let (mount_idx, relative_path) = resolve_path(cwd, path)?;

    // Handle root of mount specially.
    if relative_path.is_empty() {
        return Ok(Stat::new(0, true, FAT_EPOCH_SECS, FAT_EPOCH_SECS, FAT_EPOCH_SECS));
    }

    state::with_vfs(|vfs| {
        let mount = vfs.get_mount(mount_idx).ok_or(Fat32Error::NotFound)?;
        let fat_stat = mount.fat().stat(&relative_path)?;
        // POSIX treats a trailing slash as a directory requirement.
        if requires_dir && !fat_stat.is_dir {
            return Err(Fat32Error::NotADirectory);
        }
        Ok(Stat::new(
            fat_stat.size,
            fat_stat.is_dir,
            fat_stat.atime,
            fat_stat.mtime,
            fat_stat.ctime,
        ))
    })
}

/// Sets access and/or modification times on a path.
///
/// `None` leaves that timestamp unchanged (POSIX `UTIME_OMIT`).
///
/// # Errors
///
/// - [`Fat32Error::NotFound`] if the path doesn't exist.
/// - [`Fat32Error::ReadOnly`] if the mount is read-only.
pub(crate) fn set_times(
    cwd: &str,
    path: &str,
    atime: Option<i64>,
    mtime: Option<i64>,
) -> Result<(), Fat32Error> {
    if atime.is_none() && mtime.is_none() {
        return Ok(());
    }

    // Normalization drops trailing slashes, so enforce the directory requirement first.
    if path.ends_with('/') {
        stat(cwd, path)?;
    }

    let (mount_idx, relative_path) = resolve_path(cwd, path)?;

    // Mount root has no writable time entry; nothing to do.
    if relative_path.is_empty() {
        return Ok(());
    }

    check_writable(mount_idx)?;

    state::with_vfs(|vfs| {
        let mount = vfs.get_mount(mount_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat().set_times(&relative_path, atime, mtime)
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
/// - [`Fat32Error::AlreadyExists`] if path already exists (file or directory).
/// - [`Fat32Error::NotFound`] if parent directory doesn't exist.
/// - [`Fat32Error::NotADirectory`] if a path component is not a directory.
/// - [`Fat32Error::InvalidPath`] if the last component is not a valid FAT filename
///   (e.g. contains unsupported characters) and every ancestor is a directory.
///
/// # References
///
/// - [POSIX mkdir()](https://pubs.opengroup.org/onlinepubs/9799919799/functions/mkdir.html)
pub(crate) fn mkdir(cwd: &str, path: &str) -> Result<(), Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(cwd, path)?;

    // Root of a mount always exists — return AlreadyExists (mirrors stat()).
    if relative_path.is_empty() {
        return Err(Fat32Error::AlreadyExists);
    }

    check_writable(mount_idx)?;

    state::with_vfs_mut(|vfs| {
        let mount = vfs.get_mount_mut(mount_idx).ok_or(Fat32Error::NotFound)?;
        let fat = mount.fat_mut();

        // If path already exists (file or dir), return AlreadyExists.
        if fat.stat(&relative_path).is_ok() {
            return Err(Fat32Error::AlreadyExists);
        }

        fat.mkdir(&relative_path)
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
pub(crate) fn rmdir(cwd: &str, path: &str) -> Result<(), Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(cwd, path)?;

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
pub(crate) fn unlink(cwd: &str, path: &str) -> Result<(), Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(cwd, path)?;

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
pub(crate) fn read_dir(cwd: &str, path: &str) -> Result<Vec<DirEntry>, Fat32Error> {
    let (mount_idx, relative_path) = resolve_path(cwd, path)?;

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
/// - [`Fat32Error::ReadOnly`] if the mount is read-only.
/// - [`Fat32Error::NotFound`] if `old_path` doesn't exist.
/// - [`Fat32Error::NotADirectory`] if source is a directory but destination is a file.
/// - [`Fat32Error::NotAFile`] if source is a file but destination is a directory.
/// - [`Fat32Error::NotEmpty`] if destination is a non-empty directory (via `rmdir`).
/// - [`Fat32Error::InvalidArgument`] if either path ends in a `.` or `..` component.
/// - [`Fat32Error::InvalidPath`] if paths are on different mounts, or if path resolution
///   fails (e.g. an empty path, or a `cwd` that is not absolute).
///
/// # References
///
/// - [POSIX rename()](https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html)
pub(crate) fn rename(cwd: &str, old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
    // POSIX: a trailing "."/".." component is invalid for rename. Normalization
    // strips these lexically, so guard on the raw path before resolving.
    if ends_with_dot(old_path) || ends_with_dot(new_path) {
        return Err(Fat32Error::InvalidArgument);
    }

    let (old_idx, old_rel) = resolve_path(cwd, old_path)?;
    let (new_idx, new_rel) = resolve_path(cwd, new_path)?;

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
        let fat = mount.fat();

        // Ensure source exists before applying identity-rename fast path.
        let src_stat = fat.stat(&old_rel)?;

        // POSIX: rename(path, path) is a no-op (when the path exists).
        if old_rel == new_rel {
            return Ok(());
        }

        // POSIX rename(2): if destination exists, replace it.
        // rust-fatfs returns AlreadyExists instead, so we must remove the
        // target first after validating type compatibility.
        // NOTE: unlink + rename is not atomic — if rename fails after
        // unlink, the destination is lost. Fixing this requires upstream
        // rust-fatfs changes.
        match fat.stat(&new_rel) {
            Ok(dst_stat) => {
                if src_stat.is_dir && !dst_stat.is_dir {
                    return Err(Fat32Error::NotADirectory);
                }
                if !src_stat.is_dir && dst_stat.is_dir {
                    return Err(Fat32Error::NotAFile);
                }
                if dst_stat.is_dir {
                    // POSIX: replacing a dir requires it to be empty.
                    fat.rmdir(&new_rel)?;
                } else {
                    fat.unlink(&new_rel)?;
                }
            },
            Err(Fat32Error::NotFound) => {},
            Err(e) => return Err(e),
        }

        fat.rename(&old_rel, &new_rel)
    })
}

/// Resolves and validates a new current working directory.
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
/// - [`Fat32Error::NotADirectory`] if the path is not a directory.
pub(crate) fn change_directory(cwd: &str, path: &str) -> Result<String, Fat32Error> {
    let normalized: String = normalize(cwd, path)?;
    if !normalized.is_empty() && normalized != "/" && !stat(cwd, path)?.is_dir() {
        return Err(Fat32Error::NotADirectory);
    }
    Ok(normalized)
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
pub(crate) fn normalize(cwd: &str, path: &str) -> Result<String, Fat32Error> {
    state::with_vfs(|vfs| vfs.normalize_path(path, cwd))
}

//==================================================================================================
// Internal Functions
//==================================================================================================

/// Returns `true` if the final component of `path` is `.` or `..`.
///
/// POSIX forbids these as rename operands. Trailing slashes are ignored.
fn ends_with_dot(path: &str) -> bool {
    let trimmed: &str = path.trim_end_matches('/');
    let last: &str = trimmed.rsplit('/').next().unwrap_or(trimmed);
    last == "." || last == ".."
}

/// Resolves a path through the VFS to determine which mount handles it.
///
/// # Parameters
///
/// - `path`: The path to resolve.
///
/// # Returns
///
/// A tuple of `(mount_index, relative_path)`.
fn resolve_path(cwd: &str, path: &str) -> Result<(usize, String), Fat32Error> {
    state::with_vfs_mut(|vfs| vfs.resolve(path, cwd))
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
pub(crate) fn open_with_options(
    cwd: &str,
    path: &str,
    read: bool,
    write: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
) -> Result<File, Fat32Error> {
    if truncate && !write {
        return Err(Fat32Error::InvalidArgument);
    }
    if create_new && (create || truncate) {
        return Err(Fat32Error::InvalidArgument);
    }

    let (mount_idx, relative_path) = resolve_path(cwd, path)?;

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

        // If any ancestor component is a regular file, return ENOTDIR.
        if mount.fat().has_non_directory_ancestor(&relative_path) {
            return Err(Fat32Error::NotADirectory);
        }
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
