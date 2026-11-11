// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! High-level FAT filesystem wrapper.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Fat32Error,
    fat::{
        error::map_fatfs_error,
        file::{
            FatFile,
            IntoFileInner,
        },
        storage::{
            RawMemoryStorage,
            ReadOnlyMemoryStorage,
        },
        time::{
            date_to_unix,
            datetime_to_unix,
            is_fat_timestamp,
            unix_to_datetime,
            NanvixTimeProvider,
            FAT_EPOCH_SECS,
        },
        InternalFatFs,
        ReadOnlyInternalFatFs,
    },
    FatResolvedPath,
};
use ::core::fmt;
use ::fatfs::Write;

//==================================================================================================
// Internal Enum
//==================================================================================================

/// Internal filesystem backend that wraps either a read-write or read-only storage.
enum FatInner {
    /// Read-write filesystem backed by [`RawMemoryStorage`].
    ReadWrite(InternalFatFs),
    /// Read-only filesystem backed by [`ReadOnlyMemoryStorage`].
    ReadOnly(ReadOnlyInternalFatFs),
}

/// Dispatches a closure over the inner filesystem, regardless of storage variant.
///
/// Both arms expand the same body textually. Each expansion is type-checked
/// independently against the corresponding `fatfs::FileSystem<IO, …>` type.
macro_rules! dispatch_fs {
    ($self:expr, |$fs:ident| $body:expr) => {
        match &$self.inner {
            FatInner::ReadWrite($fs) => $body,
            FatInner::ReadOnly($fs) => $body,
        }
    };
}

//==================================================================================================
// Structures
//==================================================================================================

/// High-level FAT filesystem wrapper.
///
/// Wraps a `fatfs::FileSystem` over either [`RawMemoryStorage`] (read-write) or
/// [`ReadOnlyMemoryStorage`] (read-only) and provides a clean API that returns
/// [`Fat32Error`] instead of fatfs error types.
///
/// # Description
///
/// This is the FAT backend for the VFS. Given a pointer to a memory region
/// containing a FAT image, this type allows reading and writing files.
pub struct Fat {
    /// The underlying fatfs FileSystem.
    inner: FatInner,
    /// Base pointer to the FAT image in memory (for zero-copy file access).
    base_ptr: *const u8,
}

// SAFETY: Fat is only accessed through the VFS Mutex, which ensures
// exclusive access. The raw pointer represents memory managed by the
// state module and is only used for zero-copy reads.
unsafe impl Send for Fat {}

//==================================================================================================
// Implementations
//==================================================================================================

impl Fat {
    /// Returns `true` if this filesystem is backed by read-only storage.
    fn is_readonly(&self) -> bool {
        matches!(self.inner, FatInner::ReadOnly(_))
    }

    /// Opens an existing FAT filesystem from a memory region.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the start of the FAT image in memory.
    /// - `size`: Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// A new [`Fat`] instance, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::InvalidArgument`] if `ptr` is null or `size` is zero.
    /// - [`Fat32Error::IoError`] if the FAT image is invalid or corrupted.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory region is valid, properly aligned,
    /// and remains valid for the lifetime of this [`Fat`].
    pub unsafe fn from_memory(ptr: *mut u8, size: usize) -> Result<Self, Fat32Error> {
        // SAFETY: Caller guarantees memory region validity.
        let storage: RawMemoryStorage = unsafe { RawMemoryStorage::new(ptr, size)? };
        let options = ::fatfs::FsOptions::new().time_provider(NanvixTimeProvider);
        let fs: InternalFatFs =
            ::fatfs::FileSystem::new(storage, options).map_err(map_fatfs_error)?;
        Ok(Self {
            inner: FatInner::ReadWrite(fs),
            base_ptr: ptr as *const u8,
        })
    }

    /// Opens an existing FAT filesystem from a read-only memory region.
    ///
    /// Uses [`ReadOnlyMemoryStorage`] with an immutable `*const u8` pointer,
    /// preventing mutation of the underlying memory. Write operations through
    /// the returned [`Fat`] will fail at runtime.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Immutable pointer to the start of the FAT image in memory.
    /// - `size`: Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// A new [`Fat`] instance backed by read-only storage, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::InvalidArgument`] if `ptr` is null or `size` is zero.
    /// - [`Fat32Error::IoError`] if the FAT image is invalid or corrupted.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory region is valid, properly aligned,
    /// and remains valid for the lifetime of this [`Fat`].
    pub unsafe fn from_memory_readonly(ptr: *const u8, size: usize) -> Result<Self, Fat32Error> {
        // SAFETY: Caller guarantees memory region validity.
        let storage: ReadOnlyMemoryStorage = unsafe { ReadOnlyMemoryStorage::new(ptr, size)? };
        let options = ::fatfs::FsOptions::new().time_provider(NanvixTimeProvider);
        let fs: ReadOnlyInternalFatFs =
            ::fatfs::FileSystem::new(storage, options).map_err(map_fatfs_error)?;
        Ok(Self {
            inner: FatInner::ReadOnly(fs),
            base_ptr: ptr,
        })
    }

    /// Opens a file with the specified mode.
    ///
    /// # Parameters
    ///
    /// - `path`: Path relative to the FAT root (e.g., "subdir/file.txt").
    /// - `read`: Open for reading.
    /// - `write`: Open for writing.
    /// - `create`: Create file if it doesn't exist.
    /// - `truncate`: Truncate file to zero length.
    ///
    /// # Returns
    ///
    /// A new [`FatFile`] handle, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if file doesn't exist and `create` is false.
    /// - [`Fat32Error::IoError`] if path refers to a directory.
    pub fn open(
        &self,
        path: &FatResolvedPath,
        read: bool,
        write: bool,
        create: bool,
        truncate: bool,
    ) -> Result<FatFile<'_>, Fat32Error> {
        let path: &str = path.as_str();
        // Reject mutating open modes on a read-only filesystem.
        if self.is_readonly() && (write || create || truncate) {
            return Err(Fat32Error::ReadOnly);
        }
        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();

            // fatfs writes the directory entry of a file back only when the file is flushed or
            // dropped. Write a truncation back right away, like `FatFile::truncate()` does, so
            // that it is visible before anything is written to the truncated file.
            if create {
                match root.open_file(path) {
                    Ok(mut file) => {
                        if truncate {
                            file.truncate().map_err(map_fatfs_error)?;
                            Write::flush(&mut file).map_err(map_fatfs_error)?;
                        }
                        Ok(FatFile::new(file.into_file_inner(), read, write))
                    },
                    Err(::fatfs::Error::NotFound) => {
                        let file = root.create_file(path).map_err(map_fatfs_error)?;
                        Ok(FatFile::new(file.into_file_inner(), read, write))
                    },
                    Err(e) => Err(map_fatfs_error(e)),
                }
            } else {
                let mut file = root.open_file(path).map_err(map_fatfs_error)?;
                if truncate && write {
                    file.truncate().map_err(map_fatfs_error)?;
                    Write::flush(&mut file).map_err(map_fatfs_error)?;
                }
                Ok(FatFile::new(file.into_file_inner(), read, write))
            }
        })
    }

    /// Creates a new file, failing if it already exists.
    ///
    /// Implements `O_CREAT | O_EXCL` semantics.
    ///
    /// # Parameters
    ///
    /// - `path`: Path relative to the FAT root.
    /// - `read`: Whether the file handle should allow reading.
    /// - `write`: Whether the file handle should allow writing.
    ///
    /// # Returns
    ///
    /// A new [`FatFile`] handle, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::AlreadyExists`] if file already exists.
    /// - [`Fat32Error::NotFound`] if parent directory doesn't exist.
    pub fn create_new(
        &self,
        path: &FatResolvedPath,
        read: bool,
        write: bool,
    ) -> Result<FatFile<'_>, Fat32Error> {
        let path: &str = path.as_str();
        if self.is_readonly() {
            return Err(Fat32Error::ReadOnly);
        }
        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();

            // fatfs::Dir::create_file does NOT fail if file exists - it opens it.
            // We must explicitly check for existence first.
            if root.open_file(path).is_ok() {
                return Err(Fat32Error::AlreadyExists);
            }

            let file = root.create_file(path).map_err(map_fatfs_error)?;
            Ok(FatFile::new(file.into_file_inner(), read, write))
        })
    }

    /// Sets access and/or modification times on a file.
    ///
    /// `None` leaves that timestamp unchanged (POSIX `UTIME_OMIT`). Times are
    /// Unix seconds. FAT stores access as a date only, so sub-day access
    /// precision is dropped. Directory targets are a no-op (fatfs exposes no
    /// writable directory time entry).
    ///
    /// # Errors
    ///
    /// POSIX requires `EINVAL` for timestamps unsupported by the filesystem:
    /// <https://pubs.opengroup.org/onlinepubs/9799919799/functions/utimensat.html>.
    ///
    /// - [`Fat32Error::InvalidArgument`] if a timestamp is outside the FAT range.
    /// - [`Fat32Error::ReadOnly`] if the mount is read-only.
    /// - [`Fat32Error::NotFound`] if the path does not exist.
    pub fn set_times(
        &self,
        path: &FatResolvedPath,
        atime: Option<i64>,
        mtime: Option<i64>,
    ) -> Result<(), Fat32Error> {
        let path: &str = path.as_str();
        if atime.is_none() && mtime.is_none() {
            return Ok(());
        }
        if atime
            .into_iter()
            .chain(mtime)
            .any(|secs| !is_fat_timestamp(secs))
        {
            return Err(Fat32Error::InvalidArgument);
        }
        if self.is_readonly() {
            return Err(Fat32Error::ReadOnly);
        }
        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();
            let mut file = match root.open_file(path) {
                Ok(file) => file,
                // Directories have no writable time entry in fatfs; treat as no-op.
                Err(_) if root.open_dir(path).is_ok() => return Ok(()),
                Err(e) => return Err(map_fatfs_error(e)),
            };
            // The setters are deprecated upstream but are the only write path.
            #[allow(deprecated)]
            {
                if let Some(secs) = mtime {
                    file.set_modified(unix_to_datetime(secs));
                }
                if let Some(secs) = atime {
                    file.set_accessed(unix_to_datetime(secs).date);
                }
            }
            // Flush the dir entry now so I/O errors surface here, not on drop.
            Write::flush(&mut file).map_err(map_fatfs_error)?;
            Ok(())
        })
    }

    /// Gets file/directory metadata.
    ///
    /// # Parameters
    ///
    /// - `path`: Checked path relative to the FAT root.
    ///
    /// # Returns
    ///
    /// File metadata, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if path doesn't exist.
    pub fn stat(&self, path: &FatResolvedPath) -> Result<FatStat, Fat32Error> {
        let path: &str = path.as_str();
        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();

            // POSIX pathname resolution requires a trailing slash to name a directory.
            // https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16
            let requires_dir: bool = path.ends_with('/');
            let path: &str = path.trim_end_matches('/');

            // Mount root has no directory entry; report FAT-epoch times.
            if path.is_empty() || path == "." {
                return Ok(FatStat {
                    size: 0,
                    is_dir: true,
                    atime: FAT_EPOCH_SECS,
                    mtime: FAT_EPOCH_SECS,
                    ctime: FAT_EPOCH_SECS,
                });
            }

            // Timestamps live on the directory entry, so scan the parent.
            let (parent, name) = match path.rsplit_once('/') {
                Some((parent, name)) => (parent, name),
                None => ("", path),
            };
            let dir = if parent.is_empty() {
                root
            } else {
                root.open_dir(parent).map_err(map_fatfs_error)?
            };

            for entry in dir.iter() {
                let entry = entry.map_err(map_fatfs_error)?;
                if !entry.file_name().eq_ignore_ascii_case(name)
                    && !entry.short_file_name().eq_ignore_ascii_case(name)
                {
                    continue;
                }
                let is_dir: bool = entry.is_dir();
                if requires_dir && !is_dir {
                    return Err(Fat32Error::NotADirectory);
                }
                return Ok(FatStat {
                    size: if is_dir { 0 } else { entry.len() },
                    is_dir,
                    atime: date_to_unix(entry.accessed()),
                    mtime: datetime_to_unix(entry.modified()),
                    ctime: datetime_to_unix(entry.created()),
                });
            }

            Err(Fat32Error::NotFound)
        })
    }

    /// Returns a pointer and size for zero-copy access to a file's data.
    ///
    /// If the file's clusters are stored contiguously in the FAT image,
    /// returns `Some((data_ptr, file_size))` where `data_ptr` points directly
    /// into the in-memory FAT image. Returns `None` if the file is empty,
    /// not found, or its clusters are not contiguous.
    ///
    /// # Parameters
    ///
    /// - `path`: Path relative to the FAT root.
    pub fn file_raw_region(&self, path: &FatResolvedPath) -> Option<(*const u8, usize)> {
        let path: &str = path.as_str();
        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();
            let mut file = root.open_file(path).ok()?;

            let mut first_offset: Option<u64> = None;
            let mut total_size: usize = 0;
            let mut next_expected: u64 = 0;

            for (i, extent_result) in file.extents().enumerate() {
                let extent: ::fatfs::Extent = extent_result.ok()?;
                if i == 0 {
                    first_offset = Some(extent.offset);
                } else if extent.offset != next_expected {
                    return None; // Not contiguous.
                }
                next_expected = extent.offset + extent.size as u64;
                total_size += extent.size as usize;
            }

            let offset: u64 = first_offset?;
            if total_size == 0 {
                return None;
            }

            // SAFETY: base_ptr is valid for the lifetime of the Fat instance,
            // and offset + total_size is within the FAT image bounds.
            let data_ptr: *const u8 = unsafe { self.base_ptr.add(offset as usize) };
            Some((data_ptr, total_size))
        })
    }

    /// Reads directory contents.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the directory.
    ///
    /// # Returns
    ///
    /// A vector of directory entries (excluding `.` and `..`).
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if directory doesn't exist.
    /// - [`Fat32Error::IoError`] if path is a file.
    pub fn read_dir(
        &self,
        path: &FatResolvedPath,
    ) -> Result<alloc::vec::Vec<FatDirEntry>, Fat32Error> {
        let path: &str = path.as_str();
        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();

            let dir = if path.is_empty() || path == "/" || path == "." {
                root
            } else {
                root.open_dir(path).map_err(map_fatfs_error)?
            };

            let mut entries: alloc::vec::Vec<FatDirEntry> = alloc::vec::Vec::new();
            for entry in dir.iter() {
                let entry = entry.map_err(map_fatfs_error)?;
                let name: alloc::string::String = entry.file_name();

                if name == "." || name == ".." {
                    continue;
                }

                entries.push(FatDirEntry {
                    name,
                    is_dir: entry.is_dir(),
                    size: entry.len(),
                });
            }

            Ok(entries)
        })
    }

    /// Returns `true` if any proper ancestor of `path` exists but is not a
    /// directory. Disambiguates fatfs's `InvalidPath` (which conflates "bad
    /// path" with "non-directory component in the traversal") into a proper
    /// `NotADirectory` for POSIX `ENOTDIR` semantics. Walks from the immediate
    /// parent up to the root, stopping at the first existing non-directory.
    pub fn has_non_directory_ancestor(&self, path: &FatResolvedPath) -> bool {
        let path: &str = path.as_str();
        let mut end: usize = path.len();
        while let Some(slash) = path[..end].rfind('/') {
            let prefix: FatResolvedPath =
                match FatResolvedPath::new(alloc::string::String::from(&path[..slash])) {
                    Ok(prefix) => prefix,
                    Err(_) => return false,
                };
            if let Ok(stat) = self.stat(&prefix) {
                if !stat.is_dir {
                    return true;
                }
            }
            end = slash;
        }
        false
    }

    /// Creates a directory.
    ///
    /// # Parameters
    ///
    /// - `path`: Path for the new directory.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::AlreadyExists`] if directory already exists.
    /// - [`Fat32Error::NotFound`] if parent directory doesn't exist.
    pub fn mkdir(&self, path: &FatResolvedPath) -> Result<(), Fat32Error> {
        let path_str: &str = path.as_str();
        // Empty path means the caller resolved to the root — reject
        // before reaching fatfs, which would panic on an empty name.
        if path_str.is_empty() {
            return Err(Fat32Error::NotFound);
        }
        if self.is_readonly() {
            return Err(Fat32Error::ReadOnly);
        }

        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();

            // fatfs::Dir::create_dir does NOT fail if the directory exists — it
            // silently opens it. Check explicitly so callers get AlreadyExists.
            if root.open_dir(path_str).is_ok() {
                return Err(Fat32Error::AlreadyExists);
            }

            root.create_dir(path_str)
                .map_err(|e| match map_fatfs_error(e) {
                    Fat32Error::InvalidPath if self.has_non_directory_ancestor(path) => {
                        Fat32Error::NotADirectory
                    },
                    e => e,
                })?;
            Ok(())
        })
    }

    /// Removes an empty directory.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the directory to remove.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if directory doesn't exist.
    /// - [`Fat32Error::NotEmpty`] if directory is not empty.
    /// - [`Fat32Error::NotADirectory`] if path is a file.
    pub fn rmdir(&self, path: &FatResolvedPath) -> Result<(), Fat32Error> {
        let path: &str = path.as_str();
        // Empty path means the caller resolved to the root — reject
        // before reaching fatfs, which would panic on an empty name.
        if path.is_empty() {
            return Err(Fat32Error::NotFound);
        }
        if self.is_readonly() {
            return Err(Fat32Error::ReadOnly);
        }

        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();

            // Verify it is a directory, not a file.
            if root.open_file(path).is_ok() {
                return Err(Fat32Error::NotADirectory);
            }

            // Verify directory exists before removing.
            root.open_dir(path).map_err(map_fatfs_error)?;

            root.remove(path).map_err(map_fatfs_error)
        })
    }

    /// Deletes a file.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the file to delete.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if file doesn't exist.
    /// - [`Fat32Error::NotAFile`] if path is a directory.
    pub fn unlink(&self, path: &FatResolvedPath) -> Result<(), Fat32Error> {
        let path: &str = path.as_str();
        // Empty path means the caller resolved to the root — reject
        // before reaching fatfs, which would panic on an empty name.
        if path.is_empty() {
            return Err(Fat32Error::NotFound);
        }
        if self.is_readonly() {
            return Err(Fat32Error::ReadOnly);
        }

        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();

            // Verify it is a file, not a directory.
            if root.open_dir(path).is_ok() {
                return Err(Fat32Error::NotAFile);
            }

            // Verify file exists before removing.
            root.open_file(path).map_err(map_fatfs_error)?;

            root.remove(path).map_err(map_fatfs_error)
        })
    }

    /// Renames/moves a file or directory.
    ///
    /// # Parameters
    ///
    /// - `old_path`: Current path of the file or directory.
    /// - `new_path`: New path for the file or directory.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if source doesn't exist.
    /// - [`Fat32Error::AlreadyExists`] if destination already exists.
    pub fn rename(
        &self,
        old_path: &FatResolvedPath,
        new_path: &FatResolvedPath,
    ) -> Result<(), Fat32Error> {
        let old_path: &str = old_path.as_str();
        let new_path: &str = new_path.as_str();
        // Empty path means the caller resolved to the root — reject
        // before reaching fatfs, which would panic on an empty name.
        if old_path.is_empty() || new_path.is_empty() {
            return Err(Fat32Error::NotFound);
        }
        if self.is_readonly() {
            return Err(Fat32Error::ReadOnly);
        }

        dispatch_fs!(self, |fs| {
            let root = fs.root_dir();
            root.rename(old_path, &root, new_path)
                .map_err(map_fatfs_error)
        })
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Debug for Fat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fat").finish_non_exhaustive()
    }
}

//==================================================================================================
// Supporting Types
//==================================================================================================

/// File metadata from a FAT filesystem.
///
/// Returned by [`Fat::stat()`] to describe a file or directory.
#[derive(Debug, Clone, Copy)]
pub struct FatStat {
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// True if this is a directory.
    pub is_dir: bool,
    /// Last access time (Unix seconds). FAT stores this date-only.
    pub atime: i64,
    /// Last modification time (Unix seconds).
    pub mtime: i64,
    /// Creation time (Unix seconds).
    pub ctime: i64,
}

/// Directory entry from a FAT filesystem.
///
/// Returned by [`Fat::read_dir()`] for each file or subdirectory.
/// Does not include `.` or `..` pseudo-entries.
#[derive(Debug, Clone)]
pub struct FatDirEntry {
    /// Entry name (filename only, not full path).
    pub name: alloc::string::String,
    /// True if this is a directory.
    pub is_dir: bool,
    /// Size in bytes (0 for directories).
    pub size: u64,
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
#[allow(clippy::expect_used)]
mod tests;

//==================================================================================================
// Unit Tests (Read-Only)
//==================================================================================================

#[cfg(all(test, feature = "std"))]
#[allow(clippy::expect_used)]
mod readonly_tests;
