// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Unified file handle and POSIX-like filesystem operations.
//!
//! This module provides:
//! - [`File`]: A unified file handle for FAT filesystem files.
//! - [`OpenOptions`]: A builder for opening files with specific access modes.
//! - Free functions for filesystem operations: [`open()`], [`stat()`],
//!   [`mkdir()`], [`rmdir()`], [`unlink()`], [`rename()`], [`read_dir()`],
//!   [`chdir()`], [`cwd()`].

//==================================================================================================
// Imports
//==================================================================================================

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::FsError;
use crate::fat::FatFile;
use crate::state;

//==================================================================================================
// Constants
//==================================================================================================

/// Seek from the beginning of the file.
pub const SEEK_SET: i32 = 0;

/// Seek from the current position.
pub const SEEK_CUR: i32 = 1;

/// Seek from the end of the file.
pub const SEEK_END: i32 = 2;

//==================================================================================================
// OpenOptions
//==================================================================================================

/// Builder for opening files with specific access options.
///
/// Provides a readable, builder-pattern API for specifying file open modes.
///
/// # Default Behavior
///
/// If you call `open()` without setting any options, it defaults to read-only
/// mode (equivalent to `.read(true)`).
///
/// # Description
///
/// ```ignore
/// use fat32::{OpenOptions, File};
///
/// // Open for reading (implicit default)
/// let file = OpenOptions::new().open("/data/config.txt")?;
///
/// // Create a new file for writing
/// let file = OpenOptions::new()
///     .write(true)
///     .create(true)
///     .open("/data/output.txt")?;
///
/// // Create new file, fail if exists (O_CREAT | O_EXCL)
/// let file = OpenOptions::new()
///     .write(true)
///     .create_new(true)
///     .open("/data/unique.txt")?;
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
}

impl OpenOptions {
    /// Creates a new `OpenOptions` with all options set to false.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            read: false,
            write: false,
            create: false,
            create_new: false,
            truncate: false,
        }
    }

    /// Sets the option for read access.
    #[must_use]
    pub const fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    /// Sets the option for write access.
    #[must_use]
    pub const fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    /// Sets the option to create a new file if it doesn't exist.
    #[must_use]
    pub const fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Sets the option to truncate the file to zero length on open.
    ///
    /// Requires `write(true)`.
    #[must_use]
    pub const fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Sets the option to create a new file, failing if it already exists.
    ///
    /// This is equivalent to `O_CREAT | O_EXCL` in POSIX terms.
    #[must_use]
    pub const fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }

    /// Opens the file at the specified path with the configured options.
    ///
    /// If neither `read` nor `write` is set, defaults to `read(true)`.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the file to open.
    ///
    /// # Returns
    ///
    /// A new [`File`] handle, or an error.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotInitialized`] if the filesystem hasn't been
    ///   initialized.
    /// - [`FsError::NotFound`] if the path doesn't exist and `create` is
    ///   false.
    /// - [`FsError::ReadOnly`] if write/create/truncate on a read-only mount.
    /// - [`FsError::InvalidArgument`] if `truncate` is set without `write`,
    ///   or if `create_new` is combined with `create` or `truncate`.
    /// - [`FsError::AlreadyExists`] if `create_new` is set and file exists.
    pub fn open(self, path: &str) -> Result<File, FsError> {
        // Validate: truncate requires write.
        if self.truncate && !self.write {
            return Err(FsError::InvalidArgument);
        }

        // Validate: create_new is mutually exclusive with create and truncate.
        if self.create_new && (self.create || self.truncate) {
            return Err(FsError::InvalidArgument);
        }

        // Default to read if neither read nor write specified.
        let read: bool = if !self.read && !self.write {
            true
        } else {
            self.read
        };

        open_with_options(
            path,
            read,
            self.write,
            self.create,
            self.create_new,
            self.truncate,
        )
    }
}

//==================================================================================================
// File
//==================================================================================================

/// An open file handle on a FAT filesystem.
///
/// Provides POSIX-like read, write, and seek operations.
///
/// # Lifetime
///
/// The `'static` lifetime on the inner `FatFile` is safe because the
/// FAT filesystem backing memory is mapped before guest execution and
/// lives for the program's entire lifetime.
///
/// # Description
///
/// ```ignore
/// use fat32;
///
/// let mut file = fat32::open("/data/hello.txt")?;
/// let mut buf = [0u8; 256];
/// let n = file.read(&mut buf)?;
/// ```
pub struct File {
    /// The underlying FAT file handle.
    inner: FatFile<'static>,
    /// The mount path this file belongs to (for open file tracking).
    mount_path: String,
}

impl File {
    /// Returns true if this file supports writing.
    pub fn is_writable(&self) -> bool {
        self.inner.can_write()
    }

    /// Returns true if this file supports reading.
    pub fn is_readable(&self) -> bool {
        self.inner.can_read()
    }

    /// Reads data from the file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The buffer to read data into.
    ///
    /// # Returns
    ///
    /// The number of bytes read, or 0 at EOF.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotSupported`] if file is not open for reading.
    /// - [`FsError::IoError`] on read failure.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        self.inner.read(buf)
    }

    /// Writes data to the file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The data to write.
    ///
    /// # Returns
    ///
    /// The number of bytes written.
    ///
    /// # Errors
    ///
    /// - [`FsError::ReadOnly`] if file is not open for writing.
    /// - [`FsError::NoSpace`] if filesystem is full.
    /// - [`FsError::IoError`] on write failure.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        self.inner.write(buf)
    }

    /// Seeks to a position in the file.
    ///
    /// # Parameters
    ///
    /// - `whence`: Seek mode (`SEEK_SET`, `SEEK_CUR`, or `SEEK_END`).
    /// - `offset`: Offset in bytes.
    ///
    /// # Returns
    ///
    /// The new file position.
    ///
    /// # Errors
    ///
    /// - [`FsError::InvalidArgument`] if `whence` is invalid or offset is
    ///   negative for `SEEK_SET`.
    /// - [`FsError::IoError`] if seeking to an invalid position.
    pub fn seek(
        &mut self,
        whence: i32,
        offset: i64,
    ) -> Result<u64, FsError> {
        let pos: ::fatfs::SeekFrom = match whence {
            SEEK_SET => {
                if offset < 0 {
                    return Err(FsError::InvalidArgument);
                }
                ::fatfs::SeekFrom::Start(offset as u64)
            }
            SEEK_CUR => ::fatfs::SeekFrom::Current(offset),
            SEEK_END => ::fatfs::SeekFrom::End(offset),
            _ => return Err(FsError::InvalidArgument),
        };
        self.inner.seek(pos)
    }

    /// Flushes any buffered data to the filesystem.
    ///
    /// # Errors
    ///
    /// - [`FsError::IoError`] on flush failure.
    pub fn flush(&mut self) -> Result<(), FsError> {
        self.inner.flush()
    }

    /// Gets the file size in bytes.
    ///
    /// # Errors
    ///
    /// - [`FsError::IoError`] if seeking fails.
    pub fn size(&mut self) -> Result<u64, FsError> {
        self.inner.len()
    }

    /// Truncates the file at the current position.
    ///
    /// # Errors
    ///
    /// - [`FsError::ReadOnly`] if file is not open for writing.
    /// - [`FsError::IoError`] on truncate failure.
    pub fn truncate(&mut self) -> Result<(), FsError> {
        self.inner.truncate()
    }

    /// Reads the entire file contents into a newly allocated `Vec`.
    ///
    /// Seeks to the beginning first, then reads until EOF.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the file contents.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotSupported`] if file is not open for reading.
    /// - [`FsError::IoError`] on read failure.
    pub fn read_to_vec(&mut self) -> Result<Vec<u8>, FsError> {
        let file_size: u64 = self.inner.seek(
            ::fatfs::SeekFrom::End(0),
        )?;
        self.inner.seek(::fatfs::SeekFrom::Start(0))?;

        let mut buf: Vec<u8> = alloc::vec![0u8; file_size as usize];
        let mut total_read: usize = 0;

        while total_read < buf.len() {
            let n: usize =
                self.inner.read(&mut buf[total_read..])?;
            if n == 0 {
                break;
            }
            total_read += n;
        }

        buf.truncate(total_read);
        Ok(buf)
    }
}

impl core::fmt::Debug for File {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("File")
            .field("mount_path", &self.mount_path)
            .field("writable", &self.is_writable())
            .finish_non_exhaustive()
    }
}

impl Drop for File {
    fn drop(&mut self) {
        state::decrement_open_count(&self.mount_path);
    }
}

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
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`FsError::NotFound`] if the path doesn't exist.
pub fn open(path: &str) -> Result<File, FsError> {
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
    let (mount_idx, relative_path) = resolve_path(path).ok()?;
    let vfs = state::vfs().ok()?;
    let mount = vfs.get_mount(mount_idx)?;
    mount.fat().file_raw_region(&relative_path)
}

/// File metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    /// Size of the file in bytes (0 for directories).
    pub size: u64,
    /// Whether this is a directory.
    pub is_dir: bool,
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
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`FsError::NotFound`] if the path doesn't exist.
pub fn stat(path: &str) -> Result<Stat, FsError> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // Handle root of mount specially.
    if relative_path.is_empty() {
        return Ok(Stat {
            size: 0,
            is_dir: true,
        });
    }

    let vfs = state::vfs()?;
    let mount = vfs.get_mount(mount_idx).ok_or(FsError::NotFound)?;

    let fat_stat = mount.fat().stat(&relative_path)?;
    Ok(Stat {
        size: fat_stat.size,
        is_dir: fat_stat.is_dir,
    })
}

/// Directory entry returned by [`read_dir()`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// Name of the entry (just the filename, not full path).
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Size in bytes (0 for directories).
    pub size: u64,
}

/// Creates a directory.
///
/// # Parameters
///
/// - `path`: The path to the directory to create.
///
/// # Errors
///
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`FsError::AlreadyExists`] if directory already exists.
/// - [`FsError::NotFound`] if parent directory doesn't exist.
pub fn mkdir(path: &str) -> Result<(), FsError> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // SAFETY: Single-threaded guest, no other VFS refs held.
    let vfs = unsafe { state::vfs_mut()? };
    let mount = vfs.get_mount_mut(mount_idx).ok_or(FsError::NotFound)?;

    mount.fat_mut().mkdir(&relative_path)
}

/// Removes an empty directory.
///
/// # Parameters
///
/// - `path`: The path to the directory to remove.
///
/// # Errors
///
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`FsError::NotFound`] if directory doesn't exist.
/// - [`FsError::NotEmpty`] if directory is not empty.
/// - [`FsError::NotADirectory`] if path is a file.
pub fn rmdir(path: &str) -> Result<(), FsError> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // SAFETY: Single-threaded guest, no other VFS refs held.
    let vfs = unsafe { state::vfs_mut()? };
    let mount = vfs.get_mount_mut(mount_idx).ok_or(FsError::NotFound)?;

    mount.fat_mut().rmdir(&relative_path)
}

/// Deletes a file.
///
/// # Parameters
///
/// - `path`: The path to the file to delete.
///
/// # Errors
///
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`FsError::NotFound`] if file doesn't exist.
/// - [`FsError::NotAFile`] if path is a directory.
pub fn unlink(path: &str) -> Result<(), FsError> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // SAFETY: Single-threaded guest, no other VFS refs held.
    let vfs = unsafe { state::vfs_mut()? };
    let mount = vfs.get_mount_mut(mount_idx).ok_or(FsError::NotFound)?;

    mount.fat_mut().unlink(&relative_path)
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
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`FsError::NotFound`] if the path doesn't exist.
/// - [`FsError::NotADirectory`] if the path is a file.
pub fn read_dir(path: &str) -> Result<Vec<DirEntry>, FsError> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    let vfs = state::vfs()?;
    let mount = vfs.get_mount(mount_idx).ok_or(FsError::NotFound)?;

    let fat_path: &str = if relative_path.is_empty() {
        "."
    } else {
        &relative_path
    };

    let fat_entries = mount.fat().read_dir(fat_path)?;

    let entries: Vec<DirEntry> = fat_entries
        .into_iter()
        .map(|e| DirEntry {
            name: e.name,
            is_dir: e.is_dir,
            size: e.size,
        })
        .collect();

    Ok(entries)
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
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`FsError::NotFound`] if `old_path` doesn't exist.
/// - [`FsError::AlreadyExists`] if `new_path` already exists.
/// - [`FsError::InvalidPath`] if paths are on different mounts.
pub fn rename(
    old_path: &str,
    new_path: &str,
) -> Result<(), FsError> {
    let (old_idx, old_rel) = resolve_path(old_path)?;
    let (new_idx, new_rel) = resolve_path(new_path)?;

    // Both must be on the same mount.
    if old_idx != new_idx {
        return Err(FsError::InvalidPath);
    }

    let vfs = state::vfs()?;
    let mount = vfs.get_mount(old_idx).ok_or(FsError::NotFound)?;

    mount.fat().rename(&old_rel, &new_rel)
}

/// Gets the current working directory.
///
/// # Returns
///
/// The absolute path of the current working directory.
///
/// # Errors
///
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
pub fn cwd() -> Result<String, FsError> {
    let vfs = state::vfs()?;
    Ok(String::from(vfs.cwd()))
}

/// Changes the current working directory.
///
/// # Parameters
///
/// - `path`: The new working directory path (absolute or relative).
///
/// # Errors
///
/// - [`FsError::NotInitialized`] if the filesystem hasn't been initialized.
/// - [`FsError::InvalidPath`] if the path is malformed.
/// - [`FsError::NotFound`] if no mount handles this path.
pub fn chdir(path: &str) -> Result<(), FsError> {
    // SAFETY: Called from single-threaded guest context.
    let vfs = unsafe { state::vfs_mut()? };
    vfs.set_cwd(path)
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
fn resolve_path(
    path: &str,
) -> Result<(usize, String), FsError> {
    let vfs = state::vfs()?;
    vfs.resolve(path)
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
) -> Result<File, FsError> {
    let (mount_idx, relative_path) = resolve_path(path)?;

    // Get the mount path for open file tracking.
    let vfs = state::vfs()?;
    let mount = vfs.get_mount(mount_idx).ok_or(FsError::NotFound)?;
    let mount_path: String = String::from(mount.path());

    // SAFETY: Single-threaded guest, no other VFS refs held.
    let vfs_mut = unsafe { state::vfs_mut()? };
    let mount =
        vfs_mut.get_mount_mut(mount_idx).ok_or(FsError::NotFound)?;

    let fat_file: FatFile<'static> = if create_new {
        mount.fat_mut().create_new(&relative_path)?
    } else {
        mount
            .fat_mut()
            .open(&relative_path, read, write, create, truncate)?
    };

    // Track the open file.
    state::increment_open_count(&mount_path);

    Ok(File {
        inner: fat_file,
        mount_path,
    })
}
