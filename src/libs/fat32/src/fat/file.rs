// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! FAT file handle implementation.
//!
//! Provides a `FatFile` type that wraps a `fatfs::File` and tracks
//! read/write permissions. This type is deliberately marked `!Send + !Sync`
//! to prevent accidental sharing across threads.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Fat32Error,
    fat::{
        error::map_fatfs_error,
        InternalFatFile,
    },
};
use ::core::{
    fmt,
    marker::PhantomData,
};
use ::fatfs::{
    Read,
    Seek,
    SeekFrom,
    Write,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A file handle for a file on a FAT filesystem.
///
/// Wraps a `fatfs::File` and tracks read/write permissions.
///
/// # Thread Safety
///
/// This type is intentionally `!Send + !Sync` because file handles have
/// mutable internal state (position, buffers) and the FAT filesystem itself
/// is not thread-safe. The `PhantomData<*const ()>` marker ensures that if
/// multi-threading is ever added to guests, code that tries to share file
/// handles across threads will fail to compile.
pub struct FatFile<'a> {
    /// The underlying fatfs file.
    file: InternalFatFile<'a>,
    /// Whether this file is open for reading.
    can_read: bool,
    /// Whether this file is open for writing.
    can_write: bool,
    /// Marker to make this type `!Send + !Sync`.
    _not_send_sync: PhantomData<*const ()>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<'a> FatFile<'a> {
    /// Creates a new file handle.
    ///
    /// # Parameters
    ///
    /// - `file`: The underlying fatfs file handle.
    /// - `can_read`: Whether the file is open for reading.
    /// - `can_write`: Whether the file is open for writing.
    pub(super) fn new(file: InternalFatFile<'a>, can_read: bool, can_write: bool) -> Self {
        Self {
            file,
            can_read,
            can_write,
            _not_send_sync: PhantomData,
        }
    }

    /// Returns true if this file is open for reading.
    #[inline]
    #[must_use]
    pub fn can_read(&self) -> bool {
        self.can_read
    }

    /// Returns true if this file is open for writing.
    #[inline]
    #[must_use]
    pub fn can_write(&self) -> bool {
        self.can_write
    }

    /// Gets the current file size in bytes.
    ///
    /// # Returns
    ///
    /// The file size, or an error if seeking fails.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] if seeking fails.
    pub fn len(&mut self) -> Result<u64, Fat32Error> {
        let current: u64 = self
            .file
            .seek(SeekFrom::Current(0))
            .map_err(map_fatfs_error)?;
        let size: u64 = self.file.seek(SeekFrom::End(0)).map_err(map_fatfs_error)?;
        self.file
            .seek(SeekFrom::Start(current))
            .map_err(map_fatfs_error)?;
        Ok(size)
    }

    /// Returns true if the file is empty.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] if determining size fails.
    pub fn is_empty(&mut self) -> Result<bool, Fat32Error> {
        Ok(self.len()? == 0)
    }

    /// Reads data from the file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The buffer to read data into.
    ///
    /// # Returns
    ///
    /// The number of bytes read, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::PermissionDenied`] if file is not open for reading.
    /// - [`Fat32Error::IoError`] on read failure.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Fat32Error> {
        if !self.can_read {
            return Err(Fat32Error::PermissionDenied);
        }
        self.file.read(buf).map_err(map_fatfs_error)
    }

    /// Writes data to the file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The data to write.
    ///
    /// # Returns
    ///
    /// The number of bytes written, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::ReadOnly`] if file is not open for writing.
    /// - [`Fat32Error::NoSpace`] if filesystem is full.
    /// - [`Fat32Error::IoError`] on write failure.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Fat32Error> {
        if !self.can_write {
            return Err(Fat32Error::ReadOnly);
        }
        self.file.write(buf).map_err(map_fatfs_error)
    }

    /// Seeks to a position in the file.
    ///
    /// # Parameters
    ///
    /// - `pos`: The target seek position.
    ///
    /// # Returns
    ///
    /// The new file position, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] if seeking to an invalid position.
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Fat32Error> {
        self.file.seek(pos).map_err(map_fatfs_error)
    }

    /// Flushes any buffered data to the filesystem.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] on flush failure.
    pub fn flush(&mut self) -> Result<(), Fat32Error> {
        self.file.flush().map_err(map_fatfs_error)
    }

    /// Truncates the file at the current position.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::ReadOnly`] if file is not open for writing.
    /// - [`Fat32Error::IoError`] on truncate failure.
    pub fn truncate(&mut self) -> Result<(), Fat32Error> {
        if !self.can_write {
            return Err(Fat32Error::ReadOnly);
        }
        self.file.truncate().map_err(map_fatfs_error)
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Debug for FatFile<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FatFile")
            .field("can_read", &self.can_read)
            .field("can_write", &self.can_write)
            .finish_non_exhaustive()
    }
}
