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
        file::FatFile,
        storage::RawMemoryStorage,
        time::NanvixTimeProvider,
        InternalFatFs,
    },
};
use ::core::fmt;
use ::fatfs::{
    Seek,
    SeekFrom,
};

//==================================================================================================
// Structures
//==================================================================================================

/// High-level FAT filesystem wrapper.
///
/// Wraps a `fatfs::FileSystem` over [`RawMemoryStorage`] and provides a
/// clean API that returns [`Fat32Error`] instead of fatfs error types.
///
/// # Description
///
/// This is the FAT backend for the VFS. Given a pointer to a memory region
/// containing a FAT image, this type allows reading and writing files.
pub struct Fat {
    /// The underlying fatfs FileSystem.
    fs: InternalFatFs,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Fat {
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
        Ok(Self { fs })
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
        path: &str,
        read: bool,
        write: bool,
        create: bool,
        truncate: bool,
    ) -> Result<FatFile<'_>, Fat32Error> {
        let root = self.fs.root_dir();

        if create {
            match root.open_file(path) {
                Ok(mut file) => {
                    if truncate {
                        file.truncate().map_err(map_fatfs_error)?;
                    }
                    Ok(FatFile::new(file, read, write))
                },
                Err(::fatfs::Error::NotFound) => {
                    let file = root.create_file(path).map_err(map_fatfs_error)?;
                    Ok(FatFile::new(file, read, write))
                },
                Err(e) => Err(map_fatfs_error(e)),
            }
        } else {
            let mut file = root.open_file(path).map_err(map_fatfs_error)?;
            if truncate && write {
                file.truncate().map_err(map_fatfs_error)?;
            }
            Ok(FatFile::new(file, read, write))
        }
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
        path: &str,
        read: bool,
        write: bool,
    ) -> Result<FatFile<'_>, Fat32Error> {
        let root = self.fs.root_dir();

        // fatfs::Dir::create_file does NOT fail if file exists - it opens it.
        // We must explicitly check for existence first.
        if root.open_file(path).is_ok() {
            return Err(Fat32Error::AlreadyExists);
        }

        let file = root.create_file(path).map_err(map_fatfs_error)?;
        Ok(FatFile::new(file, read, write))
    }

    /// Gets file/directory metadata.
    ///
    /// # Parameters
    ///
    /// - `path`: Path relative to the FAT root.
    ///
    /// # Returns
    ///
    /// File metadata, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if path doesn't exist.
    pub fn stat(&self, path: &str) -> Result<FatStat, Fat32Error> {
        let root = self.fs.root_dir();

        if path.is_empty() || path == "/" || path == "." {
            return Ok(FatStat {
                size: 0,
                is_dir: true,
            });
        }

        // Try opening as file first.
        if let Ok(mut file) = root.open_file(path) {
            let size: u64 = file.seek(SeekFrom::End(0)).map_err(map_fatfs_error)?;
            return Ok(FatStat {
                size,
                is_dir: false,
            });
        }

        // Try opening as directory.
        if root.open_dir(path).is_ok() {
            return Ok(FatStat {
                size: 0,
                is_dir: true,
            });
        }

        Err(Fat32Error::NotFound)
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
    pub fn read_dir(&self, path: &str) -> Result<alloc::vec::Vec<FatDirEntry>, Fat32Error> {
        let root = self.fs.root_dir();

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
    pub fn mkdir(&self, path: &str) -> Result<(), Fat32Error> {
        let root = self.fs.root_dir();

        // fatfs::Dir::create_dir does NOT fail if the directory exists — it
        // silently opens it. Check explicitly so callers get AlreadyExists.
        if root.open_dir(path).is_ok() {
            return Err(Fat32Error::AlreadyExists);
        }

        root.create_dir(path).map_err(map_fatfs_error)?;
        Ok(())
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
    pub fn rmdir(&self, path: &str) -> Result<(), Fat32Error> {
        let root = self.fs.root_dir();

        // Verify it is a directory, not a file.
        if root.open_file(path).is_ok() {
            return Err(Fat32Error::NotADirectory);
        }

        // Verify directory exists before removing.
        root.open_dir(path).map_err(map_fatfs_error)?;

        root.remove(path).map_err(map_fatfs_error)
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
    pub fn unlink(&self, path: &str) -> Result<(), Fat32Error> {
        let root = self.fs.root_dir();

        // Verify it is a file, not a directory.
        if root.open_dir(path).is_ok() {
            return Err(Fat32Error::NotAFile);
        }

        // Verify file exists before removing.
        root.open_file(path).map_err(map_fatfs_error)?;

        root.remove(path).map_err(map_fatfs_error)
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
    pub fn rename(&self, old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
        let root = self.fs.root_dir();
        root.rename(old_path, &root, new_path)
            .map_err(map_fatfs_error)
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
