// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! FAT file handle implementation.
//!
//! Provides a `FatFile` type that wraps a `fatfs::File`, tracks read/write
//! permissions, writes the directory entry of the file back on every change,
//! and caches the file size. This type is deliberately marked `!Send + !Sync`
//! to prevent accidental sharing across threads.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Fat32Error,
    fat::{
        epoch::Epoch,
        error::map_fatfs_error,
        InternalFatFile,
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
    Write,
};

//==================================================================================================
// Internal Enum
//==================================================================================================

/// Internal file handle that wraps either a read-write or read-only `fatfs::File`.
pub(super) enum FatFileInner<'a> {
    /// Wraps a file from a read-write FAT filesystem.
    ReadWrite(InternalFatFile<'a>),
    /// Wraps a file from a read-only FAT filesystem.
    ReadOnly(ReadOnlyInternalFatFile<'a>),
}

/// Conversion trait for wrapping a `fatfs::File` into [`FatFileInner`].
pub(super) trait IntoFileInner<'a> {
    /// Wraps this `fatfs::File` into the appropriate [`FatFileInner`] variant.
    fn into_file_inner(self) -> FatFileInner<'a>;
}

impl<'a> IntoFileInner<'a> for InternalFatFile<'a> {
    fn into_file_inner(self) -> FatFileInner<'a> {
        FatFileInner::ReadWrite(self)
    }
}

impl<'a> IntoFileInner<'a> for ReadOnlyInternalFatFile<'a> {
    fn into_file_inner(self) -> FatFileInner<'a> {
        FatFileInner::ReadOnly(self)
    }
}

/// Dispatches a method call on `FatFileInner`, executing the same body
/// regardless of the storage variant.
macro_rules! dispatch_file {
    ($self:expr, |$file:ident| $body:expr) => {
        match &mut $self.inner {
            FatFileInner::ReadWrite($file) => $body,
            FatFileInner::ReadOnly($file) => $body,
        }
    };
}

//==================================================================================================
// Structures
//==================================================================================================

/// A file handle for a file on a FAT filesystem.
///
/// Wraps a `fatfs::File` and tracks read/write permissions.
///
/// # Directory Entry
///
/// Each `fatfs::File` works on a private copy of the directory entry of the
/// file, which holds the size of the file and its first cluster, and fatfs
/// writes this copy back to the filesystem only when the file is flushed or
/// dropped. Writes and truncations through this handle therefore write it back
/// right away, so that the data and the size of the file are visible to handles
/// opened afterwards and to `Fat::stat()` while this handle is still open.
/// Handles that are already open keep their own copies, and do not see changes
/// made through other handles.
///
/// The copy is written back to where the entry was when the file was opened.
/// Writing to the file after it is removed or renamed therefore overwrites
/// whatever is there by then, which fatfs warns can corrupt the filesystem.
///
/// # File Size
///
/// fatfs exposes the size of an open file only through seeking, which walks the
/// cluster chain of the file. This handle therefore queries the size once, the
/// first time that it is needed, and caches it. Only writes and truncations
/// through this handle change the size that it sees, and these keep the cached
/// size up to date.
///
/// # Modification Epoch
///
/// A write or a truncation through this handle modifies the filesystem without
/// going through [`Fat`](super::Fat) again, so this handle advances the
/// modification epoch of the filesystem, see [`Fat::epoch()`](super::Fat::epoch),
/// before it modifies anything.
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
    inner: FatFileInner<'a>,
    /// Whether this file is open for reading.
    can_read: bool,
    /// Whether this file is open for writing.
    can_write: bool,
    /// Cached file size in bytes, or `None` if it must be queried by seeking.
    cached_len: Option<u64>,
    /// Modification epoch of the filesystem that this file belongs to.
    epoch: &'a Epoch,
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
    /// - `inner`: The underlying fatfs file handle (read-write or read-only).
    /// - `can_read`: Whether the file is open for reading.
    /// - `can_write`: Whether the file is open for writing.
    /// - `epoch`: Modification epoch of the filesystem that the file belongs to.
    pub(super) fn new(
        inner: FatFileInner<'a>,
        can_read: bool,
        can_write: bool,
        epoch: &'a Epoch,
    ) -> Self {
        Self {
            inner,
            can_read,
            can_write,
            cached_len: None,
            epoch,
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
    /// The first call queries the size by seeking, and later calls return the
    /// cached size without seeking.
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
        let len: u64 = dispatch_file!(self, |file| {
            let current: u64 = file.seek(SeekFrom::Current(0)).map_err(map_fatfs_error)?;
            let size: u64 = file.seek(SeekFrom::End(0)).map_err(map_fatfs_error)?;
            file.seek(SeekFrom::Start(current))
                .map_err(map_fatfs_error)?;
            size
        });
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
        dispatch_file!(self, |file| { file.read(buf).map_err(map_fatfs_error) })
    }

    /// Writes data to the file.
    ///
    /// Writes the directory entry of the file back to the filesystem, so that
    /// the data is visible to handles opened afterwards.
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
        self.epoch.advance();
        let result: Result<usize, Fat32Error> =
            dispatch_file!(self, |file| { file.write(buf).map_err(map_fatfs_error) });
        // A write that ends past the end of the file grows the file up to where the write ends.
        // Drop the cached size if the write fails, so that it is queried again.
        self.cached_len = match self.cached_len {
            Some(len) if result.is_ok() => self.position().ok().map(|end| end.max(len)),
            _ => None,
        };
        self.write_back(result)
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
        dispatch_file!(self, |file| { file.seek(pos).map_err(map_fatfs_error) })
    }

    /// Flushes any buffered data to the filesystem.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] on flush failure.
    pub fn flush(&mut self) -> Result<(), Fat32Error> {
        dispatch_file!(self, |file| { file.flush().map_err(map_fatfs_error) })
    }

    /// Truncates the file at the current position.
    ///
    /// Writes the directory entry of the file back to the filesystem, so that
    /// the truncation is visible to handles opened afterwards.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::ReadOnly`] if file is not open for writing.
    /// - [`Fat32Error::IoError`] on truncate failure.
    pub fn truncate(&mut self) -> Result<(), Fat32Error> {
        if !self.can_write {
            return Err(Fat32Error::ReadOnly);
        }
        self.epoch.advance();
        let result: Result<(), Fat32Error> =
            dispatch_file!(self, |file| { file.truncate().map_err(map_fatfs_error) });
        // Truncating sets the size of the file to the current position. Drop the cached size if
        // the truncation fails, so that it is queried again.
        self.cached_len = if result.is_ok() {
            self.position().ok()
        } else {
            None
        };
        self.write_back(result)
    }

    /// Writes the directory entry of the file back to the filesystem after an
    /// operation that may have changed it.
    ///
    /// # Parameters
    ///
    /// - `result`: The result of the operation.
    ///
    /// # Returns
    ///
    /// The result of the operation, or an error if it succeeded but writing the
    /// directory entry back failed.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] if writing the directory entry back fails.
    fn write_back<T>(&mut self, result: Result<T, Fat32Error>) -> Result<T, Fat32Error> {
        // Write the entry back even if the operation failed, because fatfs may change the entry
        // before failing, for example when it allocates the first cluster of the file.
        let flushed: Result<(), Fat32Error> = self.flush();
        let value: T = result?;
        flushed?;
        Ok(value)
    }

    /// Returns the current position in the file.
    ///
    /// Seeking zero bytes from the current position does not walk the cluster
    /// chain of the file, so this is cheap.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::IoError`] if seeking fails.
    fn position(&mut self) -> Result<u64, Fat32Error> {
        dispatch_file!(self, |file| { file.seek(SeekFrom::Current(0)).map_err(map_fatfs_error) })
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
