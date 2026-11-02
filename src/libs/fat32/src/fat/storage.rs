// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Raw memory storage backend for FAT filesystem.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Fat32Error,
    fat::error::MemoryIoError,
};
use ::core::fmt;
use ::fatfs::{
    IoBase,
    Read,
    Seek,
    SeekFrom,
    Write,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A storage backend backed by a raw memory region.
///
/// Wraps a `(*mut u8, usize)` pair and implements the fatfs I/O traits,
/// allowing `fatfs::FileSystem` to read/write a FAT image in memory.
///
/// # Safety
///
/// The caller must ensure:
/// - The memory region is valid and accessible for the lifetime of this storage
/// - No concurrent access without synchronization
/// - The region is not unmapped while the storage is in use
pub struct RawMemoryStorage {
    /// Pointer to start of the memory region.
    base: *mut u8,
    /// Size of the memory region in bytes.
    size: usize,
    /// Current read/write position.
    position: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RawMemoryStorage {
    /// Creates a new storage over a memory region.
    ///
    /// # Parameters
    ///
    /// - `base`: Pointer to the start of the FAT image in memory.
    /// - `size`: Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// `Ok(Self)` on success, or `Err(Fat32Error::InvalidArgument)` if `base` is
    /// null or `size` is zero.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `base` points to a valid, readable, and writable memory region
    /// - The memory region is at least `size` bytes
    /// - The memory remains valid for the lifetime of this `RawMemoryStorage`
    /// - No other code accesses this memory region concurrently
    #[inline]
    pub unsafe fn new(base: *mut u8, size: usize) -> Result<Self, Fat32Error> {
        if base.is_null() {
            return Err(Fat32Error::InvalidArgument);
        }
        if size == 0 {
            return Err(Fat32Error::InvalidArgument);
        }
        Ok(Self {
            base,
            size,
            position: 0,
        })
    }

    /// Returns the number of bytes remaining from current position to end.
    #[inline]
    fn remaining(&self) -> usize {
        self.size.saturating_sub(self.position)
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Debug for RawMemoryStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawMemoryStorage")
            .field("base", &self.base)
            .field("size", &self.size)
            .field("position", &self.position)
            .finish()
    }
}

// SAFETY: RawMemoryStorage is only accessed through the VFS Mutex,
// which ensures exclusive access. The raw pointer represents memory
// managed by the state module.
unsafe impl Send for RawMemoryStorage {}

impl IoBase for RawMemoryStorage {
    type Error = MemoryIoError;
}

impl Read for RawMemoryStorage {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let to_read: usize = buf.len().min(self.remaining());
        if to_read == 0 {
            return Ok(0);
        }

        // SAFETY: We verified position + to_read <= size, and the caller
        // guaranteed the memory region is valid via the unsafe constructor.
        unsafe {
            let src: *const u8 = self.base.add(self.position);
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), to_read);
        }

        self.position += to_read;
        Ok(to_read)
    }
}

impl Write for RawMemoryStorage {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let to_write: usize = buf.len().min(self.remaining());
        if to_write == 0 {
            return Ok(0);
        }

        // SAFETY: We verified position + to_write <= size, and the caller
        // guaranteed the memory region is valid via the unsafe constructor.
        unsafe {
            let dst: *mut u8 = self.base.add(self.position);
            core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, to_write);
        }

        self.position += to_write;
        Ok(to_write)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        // Memory writes are immediately visible; nothing to flush.
        Ok(())
    }
}

impl Seek for RawMemoryStorage {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let new_pos: i64 = match pos {
            SeekFrom::Start(offset) => {
                i64::try_from(offset).map_err(|_| MemoryIoError::OutOfBounds)?
            },
            SeekFrom::End(offset) => {
                let size: i64 = i64::try_from(self.size).map_err(|_| MemoryIoError::OutOfBounds)?;
                size.checked_add(offset).ok_or(MemoryIoError::OutOfBounds)?
            },
            SeekFrom::Current(offset) => {
                let pos: i64 =
                    i64::try_from(self.position).map_err(|_| MemoryIoError::OutOfBounds)?;
                pos.checked_add(offset).ok_or(MemoryIoError::OutOfBounds)?
            },
        };

        if new_pos < 0 {
            return Err(MemoryIoError::InvalidSeek);
        }

        let new_pos: usize = usize::try_from(new_pos).map_err(|_| MemoryIoError::OutOfBounds)?;

        if new_pos > self.size {
            return Err(MemoryIoError::OutOfBounds);
        }

        self.position = new_pos;
        Ok(new_pos as u64)
    }
}

//==================================================================================================
// ReadOnlyMemoryStorage
//==================================================================================================

/// A read-only storage backend backed by a raw memory region.
pub struct ReadOnlyMemoryStorage {
    /// Pointer to start of the memory region (immutable).
    base: *const u8,
    /// Size of the memory region in bytes.
    size: usize,
    /// Current read position.
    position: usize,
}

//==================================================================================================
// ReadOnlyMemoryStorage Implementations
//==================================================================================================

impl ReadOnlyMemoryStorage {
    ///
    /// # Description
    ///
    /// Creates a new read-only storage over a memory region.
    ///
    /// # Parameters
    ///
    /// - `base`: Pointer to the start of the FAT image in memory.
    /// - `size`: Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// - Success: `Ok(Self)`
    /// - Failure: `Err(Fat32Error::InvalidArgument)` if `base` is null or `size` is zero.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `base` points to a valid, readable memory region.
    /// - The memory region is at least `size` bytes.
    /// - The memory remains valid for the lifetime of this `ReadOnlyMemoryStorage`.
    /// - No other code mutates this memory region concurrently.
    ///
    #[inline]
    pub unsafe fn new(base: *const u8, size: usize) -> Result<Self, Fat32Error> {
        if base.is_null() {
            return Err(Fat32Error::InvalidArgument);
        }
        if size == 0 {
            return Err(Fat32Error::InvalidArgument);
        }
        Ok(Self {
            base,
            size,
            position: 0,
        })
    }

    /// Returns the number of bytes remaining from current position to end.
    #[inline]
    fn remaining(&self) -> usize {
        self.size.saturating_sub(self.position)
    }
}

//==================================================================================================
// ReadOnlyMemoryStorage Trait Implementations
//==================================================================================================

impl fmt::Debug for ReadOnlyMemoryStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadOnlyMemoryStorage")
            .field("base", &self.base)
            .field("size", &self.size)
            .field("position", &self.position)
            .finish()
    }
}

// SAFETY: Moving ReadOnlyMemoryStorage to another thread is sound because it provides ownership-
// based access only: mutating the cursor requires `&mut self`, so sharing the same instance across
// threads still requires external synchronization by the caller. The unsafe constructor requires
// `base` to point to a valid readable memory region that remains valid for the lifetime of the
// storage, and that region must not be concurrently mutated.
unsafe impl Send for ReadOnlyMemoryStorage {}

impl IoBase for ReadOnlyMemoryStorage {
    type Error = MemoryIoError;
}

impl Read for ReadOnlyMemoryStorage {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let to_read: usize = buf.len().min(self.remaining());
        if to_read == 0 {
            return Ok(0);
        }

        // SAFETY: We verified position + to_read <= size, and the caller
        // guaranteed the memory region is valid via the unsafe constructor.
        unsafe {
            let src: *const u8 = self.base.add(self.position);
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), to_read);
        }

        self.position += to_read;
        Ok(to_read)
    }
}

impl Write for ReadOnlyMemoryStorage {
    fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> {
        Err(MemoryIoError::ReadOnly)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        // Nothing to flush on a read-only storage.
        Ok(())
    }
}

impl Seek for ReadOnlyMemoryStorage {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let new_pos: i64 = match pos {
            SeekFrom::Start(offset) => {
                i64::try_from(offset).map_err(|_| MemoryIoError::OutOfBounds)?
            },
            SeekFrom::End(offset) => {
                let size: i64 = i64::try_from(self.size).map_err(|_| MemoryIoError::OutOfBounds)?;
                size.checked_add(offset).ok_or(MemoryIoError::OutOfBounds)?
            },
            SeekFrom::Current(offset) => {
                let pos: i64 =
                    i64::try_from(self.position).map_err(|_| MemoryIoError::OutOfBounds)?;
                pos.checked_add(offset).ok_or(MemoryIoError::OutOfBounds)?
            },
        };

        if new_pos < 0 {
            return Err(MemoryIoError::InvalidSeek);
        }

        let new_pos: usize = usize::try_from(new_pos).map_err(|_| MemoryIoError::OutOfBounds)?;

        if new_pos > self.size {
            return Err(MemoryIoError::OutOfBounds);
        }

        self.position = new_pos;
        Ok(new_pos as u64)
    }
}

//==================================================================================================
// Unit Tests (RawMemoryStorage)
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

//==================================================================================================
// Unit Tests (ReadOnlyMemoryStorage)
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod readonly_tests;
