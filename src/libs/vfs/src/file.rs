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
use ::sysapi::unistd::file_seek;

//==================================================================================================
// Constants
//==================================================================================================

/// Path component separator.
const VFS_PATH_SEPARATOR: char = '/';

/// Current directory name.
const VFS_CURRENT_DIRECTORY: &str = ".";

/// Parent directory name.
const VFS_PARENT_DIRECTORY: &str = "..";

/// Root directory path.
const VFS_ROOT_DIRECTORY: &str = "/";

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
/// use vfs::{OpenOptions, File};
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
    /// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been
    ///   initialized.
    /// - [`Fat32Error::NotFound`] if the path doesn't exist and `create` is
    ///   false.
    /// - [`Fat32Error::ReadOnly`] if write/create/truncate on a read-only mount.
    /// - [`Fat32Error::InvalidArgument`] if `truncate` is set without `write`,
    ///   or if `create_new` is combined with `create` or `truncate`.
    /// - [`Fat32Error::AlreadyExists`] if `create_new` is set and file exists.
    pub fn open(self, path: &str) -> Result<File, Fat32Error> {
        // Validate: truncate requires write.
        if self.truncate && !self.write {
            return Err(Fat32Error::InvalidArgument);
        }

        // Validate: create_new is mutually exclusive with create and truncate.
        if self.create_new && (self.create || self.truncate) {
            return Err(Fat32Error::InvalidArgument);
        }

        // Default to read if neither read nor write specified.
        let read: bool = if !self.read && !self.write {
            true
        } else {
            self.read
        };

        open_with_options(path, read, self.write, self.create, self.create_new, self.truncate)
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
/// use vfs;
///
/// let mut file = vfs::open("/data/hello.txt")?;
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
    #[must_use]
    pub fn is_writable(&self) -> bool {
        self.inner.can_write()
    }

    /// Returns true if this file supports reading.
    #[must_use]
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
    /// - [`Fat32Error::PermissionDenied`] if file is not open for reading.
    /// - [`Fat32Error::IoError`] on read failure.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Fat32Error> {
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
    /// - [`Fat32Error::ReadOnly`] if file is not open for writing.
    /// - [`Fat32Error::NoSpace`] if filesystem is full.
    /// - [`Fat32Error::IoError`] on write failure.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Fat32Error> {
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
    /// - [`Fat32Error::InvalidArgument`] if `whence` is invalid or offset is
    ///   negative for `SEEK_SET`.
    /// - [`Fat32Error::IoError`] if seeking to an invalid position.
    pub fn seek(&mut self, whence: i32, offset: i64) -> Result<u64, Fat32Error> {
        let pos: ::fatfs::SeekFrom = match whence {
            file_seek::SEEK_SET => {
                if offset < 0 {
                    return Err(Fat32Error::InvalidArgument);
                }
                ::fatfs::SeekFrom::Start(offset as u64)
            },
            file_seek::SEEK_CUR => ::fatfs::SeekFrom::Current(offset),
            file_seek::SEEK_END => ::fatfs::SeekFrom::End(offset),
            _ => return Err(Fat32Error::InvalidArgument),
        };
        self.inner.seek(pos)
    }

    /// Flushes any buffered data to the filesystem.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] on flush failure.
    pub fn flush(&mut self) -> Result<(), Fat32Error> {
        self.inner.flush()
    }

    /// Gets the file size in bytes.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] if seeking fails.
    pub fn size(&mut self) -> Result<u64, Fat32Error> {
        self.inner.len()
    }

    /// Truncates the file at the current position.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::ReadOnly`] if file is not open for writing.
    /// - [`Fat32Error::IoError`] on truncate failure.
    pub fn truncate(&mut self) -> Result<(), Fat32Error> {
        self.inner.truncate()
    }

    /// Reads the entire file contents into a newly allocated `Vec`.
    ///
    /// Seeks to the beginning of the file, then reads until EOF. After
    /// returning, the file position is at the end of the data that was read.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the file contents.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::PermissionDenied`] if file is not open for reading.
    /// - [`Fat32Error::OutOfMemory`] if the file size exceeds addressable memory.
    /// - [`Fat32Error::IoError`] on read failure.
    pub fn read_to_vec(&mut self) -> Result<Vec<u8>, Fat32Error> {
        if !self.inner.can_read() {
            return Err(Fat32Error::PermissionDenied);
        }

        let file_size: u64 = self.inner.seek(::fatfs::SeekFrom::End(0))?;
        self.inner.seek(::fatfs::SeekFrom::Start(0))?;

        let buf_size: usize = usize::try_from(file_size).map_err(|_| Fat32Error::OutOfMemory)?;
        let mut buf: Vec<u8> = alloc::vec![0u8; buf_size];
        let mut total_read: usize = 0;

        while total_read < buf.len() {
            let n: usize = self.inner.read(&mut buf[total_read..])?;
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
///
/// # TODOs
///
/// - TODO (#2065): Cache negative results for read-only mounts.
///
pub fn file_raw_region(path: &str) -> Option<(*const u8, usize)> {
    // Normalize the path so the cache key is stable across cwd changes and different spellings.
    let normalized_path: String = normalize_for_cache(path).ok()?;

    // Check cache first, using the normalized path as the key.
    if let Some(cached) = crate::cache::get_raw_region(&normalized_path) {
        return Some(cached);
    }

    let (mount_idx, relative_path): (usize, String) = resolve_path(&normalized_path).ok()?;
    let result = state::with_vfs(|vfs| {
        let mount: &crate::mount::Mount = vfs.get_mount(mount_idx).ok_or(Fat32Error::NotFound)?;
        Ok(mount.fat().file_raw_region(&relative_path))
    })
    .ok()?;

    // Only cache positive results.
    if result.is_some() {
        crate::cache::put_raw_region(&normalized_path, result);
    }

    result
}

/// File metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    /// Size of the file in bytes (0 for directories).
    size: u64,
    /// Whether this is a directory.
    is_dir: bool,
}

impl Stat {
    /// Creates a new `Stat` instance.
    ///
    /// # Parameters
    ///
    /// - `size`: File size in bytes (0 for directories).
    /// - `is_dir`: Whether this entry is a directory.
    pub fn new(size: u64, is_dir: bool) -> Self {
        Self { size, is_dir }
    }

    /// Returns the file size in bytes (0 for directories).
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns whether this entry is a directory.
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
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
///
/// # TODOs
///
/// - TODO (#2065): Cache negative results for read-only mounts.
///
pub fn stat(path: &str) -> Result<Stat, Fat32Error> {
    // Normalize the path so the cache key is stable across cwd changes and different spellings.
    let normalized_path: String = normalize_for_cache(path)?;

    // Check stat cache.
    if let Some(cached) = crate::cache::get_stat(&normalized_path) {
        return Ok(cached);
    }

    let result: Result<(usize, String), Fat32Error> = resolve_path(&normalized_path);
    let (mount_idx, relative_path) = match result {
        Ok(v) => v,
        // NotFound from resolve_path means no mount matches this path,
        // not that the file is missing. Do not negative-cache here.
        Err(Fat32Error::NotFound) => return Err(Fat32Error::NotFound),
        Err(e) => return Err(e),
    };

    // Handle root of mount specially.
    if relative_path.is_empty() {
        let s = Stat::new(0, true);
        crate::cache::put_stat(&normalized_path, s);
        return Ok(s);
    }

    let result: Result<Stat, Fat32Error> = state::with_vfs(|vfs| {
        let mount = vfs.get_mount(mount_idx).ok_or(Fat32Error::NotFound)?;
        let fat_stat = mount.fat().stat(&relative_path)?;
        Ok(Stat::new(fat_stat.size, fat_stat.is_dir))
    });

    match result {
        Ok(s) => {
            crate::cache::put_stat(&normalized_path, s);
            Ok(s)
        },
        Err(Fat32Error::NotFound) => Err(Fat32Error::NotFound),
        Err(e) => Err(e),
    }
}

/// Directory entry returned by [`read_dir()`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// Name of the entry (just the filename, not full path).
    name: String,
    /// Whether this entry is a directory.
    is_dir: bool,
    /// Size in bytes (0 for directories).
    size: u64,
}

impl DirEntry {
    /// Creates a new `DirEntry` instance.
    ///
    /// # Parameters
    ///
    /// - `name`: Entry name (filename only, not full path).
    /// - `is_dir`: Whether this entry is a directory.
    /// - `size`: Size in bytes (0 for directories).
    pub fn new(name: String, is_dir: bool, size: u64) -> Self {
        Self { name, is_dir, size }
    }

    /// Returns the entry name (filename only, not full path).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether this entry is a directory.
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Returns the size in bytes (0 for directories).
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }
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

    let result = state::with_vfs_mut(|vfs| {
        let mount = vfs.get_mount_mut(mount_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat_mut().mkdir(&relative_path)
    });

    if result.is_ok() {
        if let Ok(normalized) = crate::normalize(path) {
            crate::cache::invalidate_path(&normalized);
        }
    }
    result
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

    let result = state::with_vfs_mut(|vfs| {
        let mount = vfs.get_mount_mut(mount_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat_mut().rmdir(&relative_path)
    });

    if result.is_ok() {
        if let Ok(normalized) = crate::normalize(path) {
            crate::cache::invalidate_path(&normalized);
        }
    }
    result
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

    let result = state::with_vfs_mut(|vfs| {
        let mount = vfs.get_mount_mut(mount_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat_mut().unlink(&relative_path)
    });

    if result.is_ok() {
        if let Ok(normalized) = crate::normalize(path) {
            crate::cache::invalidate_path(&normalized);
        }
    }
    result
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
            VFS_CURRENT_DIRECTORY
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

    let result: Result<(), Fat32Error> = state::with_vfs(|vfs| {
        let mount = vfs.get_mount(old_idx).ok_or(Fat32Error::NotFound)?;
        mount.fat().rename(&old_rel, &new_rel)
    });

    if result.is_ok() {
        if let Ok(old_normalized) = crate::normalize(old_path) {
            crate::cache::invalidate_path(&old_normalized);
        }
        if let Ok(new_normalized) = crate::normalize(new_path) {
            crate::cache::invalidate_path(&new_normalized);
        }
    }
    result
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
    state::with_vfs(|vfs| Ok(String::from(vfs.cwd())))
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
    state::with_vfs_mut(|vfs| vfs.set_cwd(path))
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
    state::with_vfs(|vfs| vfs.normalize_path(path))
}

//==================================================================================================
// Internal Functions
//==================================================================================================

///
/// # Description
///
/// Normalizes a path for use as a cache key, avoiding the VFS lock for
/// absolute paths.
///
/// Absolute paths are normalized in-place without VFS state. Relative paths
/// fall back to [`normalize()`], which requires the CWD (and thus the VFS
/// lock).
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
/// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been initialized
///   and the path is relative.
/// - [`Fat32Error::InvalidPath`] if the path is malformed.
fn normalize_for_cache(path: &str) -> Result<String, Fat32Error> {
    if path.starts_with(VFS_PATH_SEPARATOR) {
        normalize_absolute(path)
    } else {
        normalize(path)
    }
}

///
/// # Description
///
/// Resolves `.` and `..` components in an absolute path without VFS state.
///
/// # Parameters
///
/// - `path`: The absolute path to normalize (must start with `/`).
///
/// # Returns
///
/// The normalized absolute path.
///
/// # Errors
///
/// - [`Fat32Error::InvalidPath`] if the path is empty, has too many `..`
///   components, or contains empty components (e.g., consecutive separators
///   like `//`).
fn normalize_absolute(path: &str) -> Result<String, Fat32Error> {
    if path.is_empty() {
        return Err(Fat32Error::InvalidPath);
    }

    // Strip the leading separator (guaranteed by caller) and any trailing
    // separators so they do not produce spurious empty components.
    let inner: &str = path
        .strip_prefix(VFS_PATH_SEPARATOR)
        .unwrap_or(path)
        .trim_end_matches(VFS_PATH_SEPARATOR);

    // Root path: only separators.
    if inner.is_empty() {
        return Ok(String::from(VFS_ROOT_DIRECTORY));
    }

    let mut components: Vec<&str> = Vec::new();
    for component in inner.split(VFS_PATH_SEPARATOR) {
        match component {
            "" => return Err(Fat32Error::InvalidPath),
            VFS_CURRENT_DIRECTORY => {},
            VFS_PARENT_DIRECTORY => {
                if components.pop().is_none() {
                    return Err(Fat32Error::InvalidPath);
                }
            },
            other => {
                components.push(other);
            },
        }
    }

    if components.is_empty() {
        Err(Fat32Error::InvalidPath)
    } else {
        let mut result: String = String::new();
        for component in components {
            result.push(VFS_PATH_SEPARATOR);
            result.push_str(component);
        }
        Ok(result)
    }
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
fn resolve_path(path: &str) -> Result<(usize, String), Fat32Error> {
    state::with_vfs(|vfs| vfs.resolve(path))
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
    // NOTE: This gate also preserves cache consistency — cached entries are
    // only populated for read-only mounts' paths, so any O_CREAT that could
    // create a file is blocked here before it reaches the FAT layer,
    // preventing stale cache entries.
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

    Ok(File {
        inner: fat_file,
        mount_path,
    })
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- OpenOptions builder tests -----------------------------------------------

    /// Tests that default OpenOptions has all flags false.
    #[test]
    fn open_options_default() {
        let opts: OpenOptions = OpenOptions::new();
        assert!(!opts.read, "read should default to false");
        assert!(!opts.write, "write should default to false");
        assert!(!opts.create, "create should default to false");
        assert!(!opts.create_new, "create_new should default to false");
        assert!(!opts.truncate, "truncate should default to false");
    }

    /// Tests builder method chaining.
    #[test]
    fn open_options_builder_chaining() {
        let opts: OpenOptions = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true);
        assert!(opts.read, "read should be true");
        assert!(opts.write, "write should be true");
        assert!(opts.create, "create should be true");
        assert!(opts.truncate, "truncate should be true");
    }

    /// Tests that truncate without write is rejected.
    #[test]
    fn open_options_truncate_requires_write() {
        let result = OpenOptions::new().truncate(true).open("/nonexistent");
        assert_eq!(
            result.unwrap_err(),
            Fat32Error::InvalidArgument,
            "truncate without write should be rejected"
        );
    }

    /// Tests that create_new + create is rejected.
    #[test]
    fn open_options_create_new_excludes_create() {
        let result = OpenOptions::new()
            .write(true)
            .create(true)
            .create_new(true)
            .open("/nonexistent");
        assert_eq!(
            result.unwrap_err(),
            Fat32Error::InvalidArgument,
            "create_new + create should be rejected"
        );
    }

    /// Tests that create_new + truncate is rejected.
    #[test]
    fn open_options_create_new_excludes_truncate() {
        let result = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create_new(true)
            .open("/nonexistent");
        assert_eq!(
            result.unwrap_err(),
            Fat32Error::InvalidArgument,
            "create_new + truncate should be rejected"
        );
    }

    /// Tests that Default trait matches OpenOptions::new().
    #[test]
    fn open_options_implements_default() {
        let default: OpenOptions = OpenOptions::default();
        let new: OpenOptions = OpenOptions::new();
        assert_eq!(default.read, new.read);
        assert_eq!(default.write, new.write);
        assert_eq!(default.create, new.create);
        assert_eq!(default.create_new, new.create_new);
        assert_eq!(default.truncate, new.truncate);
    }

    /// Tests that OpenOptions implements Debug.
    #[test]
    fn open_options_debug() {
        let opts: OpenOptions = OpenOptions::new().read(true);
        let debug: alloc::string::String = alloc::format!("{opts:?}");
        assert!(debug.contains("OpenOptions"), "debug output should contain type name");
    }

    // -- Stat tests --------------------------------------------------------------

    /// Tests Stat equality and debug.
    #[test]
    fn stat_clone_eq_debug() {
        let s: Stat = Stat::new(42, false);
        let cloned: Stat = s;
        assert_eq!(s, cloned, "clone should preserve equality");

        let other: Stat = Stat::new(0, true);
        assert_ne!(s, other, "different stats should not be equal");

        assert_eq!(s.size(), 42, "size accessor should return 42");
        assert!(!s.is_dir(), "is_dir accessor should return false");

        let debug: alloc::string::String = alloc::format!("{s:?}");
        assert!(debug.contains("42"), "debug should contain size");
    }

    // -- DirEntry tests ----------------------------------------------------------

    /// Tests DirEntry equality and debug.
    #[test]
    fn dir_entry_clone_eq_debug() {
        let entry: DirEntry = DirEntry::new(String::from("test.txt"), false, 100);
        let cloned: DirEntry = entry.clone();
        assert_eq!(entry, cloned, "clone should preserve equality");

        assert_eq!(entry.name(), "test.txt", "name accessor should return name");
        assert!(!entry.is_dir(), "is_dir accessor should return false");
        assert_eq!(entry.size(), 100, "size accessor should return 100");

        let debug: alloc::string::String = alloc::format!("{entry:?}");
        assert!(debug.contains("test.txt"), "debug should contain name");
    }
}
