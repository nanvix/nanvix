// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Read-only FAT file handle implementation.
//!
//! Provides a `ReadOnlyFatFile` type that wraps a `fatfs::File` of a read-only FAT filesystem.
//! Unlike [`FatFile`](super::FatFile), it can only read: it has no write or truncate methods, and
//! it needs neither permission flags nor a modification epoch to tell them apart. This type is
//! deliberately marked `!Send + !Sync` to prevent accidental sharing across threads.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Fat32Error,
    fat::{
        error::map_fatfs_error,
        file::seek_len,
        ReadOnlyInternalFatFile,
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
};

//==================================================================================================
// Structures
//==================================================================================================

/// A file handle for a file on a read-only FAT filesystem.
///
/// Wraps a `fatfs::File` over [`ReadOnlyMemoryStorage`](super::ReadOnlyMemoryStorage). Only a
/// read-only [`Fat`](super::Fat) opens one, see
/// [`Fat::open_readonly()`](super::Fat::open_readonly).
///
/// # Read-Only
///
/// Nothing can change a read-only filesystem, so this handle is always readable and never
/// writable, and its type says so: it has no write or truncate methods, and it does not track
/// permissions. This sets it apart from [`FatFile`](super::FatFile), which checks what the file
/// was opened for on every read and write, and dispatches on whether its filesystem is read-write.
///
/// # File Size
///
/// fatfs exposes the size of an open file only through seeking, which walks the cluster chain of
/// the file. This handle therefore queries the size once, the first time that it is needed, and
/// caches it. The size cannot change afterwards, because the filesystem is read-only.
///
/// # Thread Safety
///
/// This type is intentionally `!Send + !Sync` because file handles have mutable internal state
/// (position, buffers) and the FAT filesystem itself is not thread-safe. The
/// `PhantomData<*const ()>` marker ensures that if multi-threading is ever added to guests, code
/// that tries to share file handles across threads will fail to compile.
pub struct ReadOnlyFatFile<'a> {
    /// The underlying fatfs file.
    inner: ReadOnlyInternalFatFile<'a>,
    /// Cached file size in bytes, or `None` if it must be queried by seeking.
    cached_len: Option<u64>,
    /// Marker to make this type `!Send + !Sync`.
    _not_send_sync: PhantomData<*const ()>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<'a> ReadOnlyFatFile<'a> {
    /// Creates a new file handle.
    ///
    /// # Parameters
    ///
    /// - `inner`: The underlying fatfs file handle of a read-only filesystem.
    pub(super) fn new(inner: ReadOnlyInternalFatFile<'a>) -> Self {
        Self {
            inner,
            cached_len: None,
            _not_send_sync: PhantomData,
        }
    }

    /// Gets the file size in bytes.
    ///
    /// The first call queries the size by seeking and restores the position, and later calls
    /// return the cached size without seeking.
    ///
    /// # Returns
    ///
    /// The file size, or an error if seeking fails.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] if seeking fails.
    pub fn len(&mut self) -> Result<u64, Fat32Error> {
        if let Some(len) = self.cached_len {
            return Ok(len);
        }
        let len: u64 = seek_len(&mut self.inner)?;
        self.cached_len = Some(len);
        Ok(len)
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
    /// The number of bytes read, or 0 at the end of the file.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] on read failure.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Fat32Error> {
        self.inner.read(buf).map_err(map_fatfs_error)
    }

    /// Seeks to a position in the file.
    ///
    /// A position past the end of the file is clamped to the end of the file.
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
    /// - [`Fat32Error::InvalidPath`] if the target position is negative or too large for a FAT
    ///   file, which is limited to 4 GiB - 1 byte.
    /// - [`Fat32Error::IoError`] if seeking fails.
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Fat32Error> {
        self.inner.seek(pos).map_err(map_fatfs_error)
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Debug for ReadOnlyFatFile<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadOnlyFatFile").finish_non_exhaustive()
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
#[allow(clippy::expect_used)]
mod tests;
