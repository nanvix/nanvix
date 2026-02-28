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
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::alloc::{
        vec,
        vec::Vec,
    };

    /// Helper: creates a `RawMemoryStorage` backed by a mutable Vec.
    /// Returns the storage and the backing buffer (which must be kept alive).
    fn make_storage(size: usize) -> (RawMemoryStorage, Vec<u8>) {
        let mut buf: Vec<u8> = vec![0u8; size];
        let ptr: *mut u8 = buf.as_mut_ptr();
        // SAFETY: ptr points to `buf` which is valid for `size` bytes.
        let storage: RawMemoryStorage =
            unsafe { RawMemoryStorage::new(ptr, size).expect("valid storage") };
        (storage, buf)
    }

    // -- Constructor tests -------------------------------------------------------

    /// Tests that null pointer is rejected.
    #[test]
    fn new_rejects_null_pointer() {
        let result: Result<RawMemoryStorage, _> =
            unsafe { RawMemoryStorage::new(core::ptr::null_mut(), 1024) };
        assert_eq!(
            result.unwrap_err(),
            Fat32Error::InvalidArgument,
            "null pointer should be rejected"
        );
    }

    /// Tests that zero size is rejected.
    #[test]
    fn new_rejects_zero_size() {
        let mut buf: [u8; 1] = [0];
        let result: Result<RawMemoryStorage, _> =
            unsafe { RawMemoryStorage::new(buf.as_mut_ptr(), 0) };
        assert_eq!(
            result.unwrap_err(),
            Fat32Error::InvalidArgument,
            "zero size should be rejected"
        );
    }

    /// Tests successful construction.
    #[test]
    fn new_succeeds_with_valid_args() {
        let (storage, _buf) = make_storage(64);
        assert_eq!(storage.remaining(), 64, "remaining should equal size at position 0");
    }

    // -- Read tests --------------------------------------------------------------

    /// Tests reading data that was written to the backing buffer.
    #[test]
    fn read_returns_data() {
        let mut buf: Vec<u8> = vec![0xAA; 16];
        let ptr: *mut u8 = buf.as_mut_ptr();
        let mut storage: RawMemoryStorage =
            unsafe { RawMemoryStorage::new(ptr, 16).expect("valid storage") };

        let mut read_buf: [u8; 4] = [0; 4];
        let n: usize = storage.read(&mut read_buf).expect("read should succeed");
        assert_eq!(n, 4, "should read 4 bytes");
        assert_eq!(read_buf, [0xAA; 4], "should read the correct data");
    }

    /// Tests reading an empty buffer.
    #[test]
    fn read_empty_buffer_returns_zero() {
        let (mut storage, _buf) = make_storage(16);
        let mut empty: [u8; 0] = [];
        let n: usize = storage.read(&mut empty).expect("read should succeed");
        assert_eq!(n, 0, "reading empty buffer should return 0");
    }

    /// Tests that read at end of storage returns 0.
    #[test]
    fn read_at_end_returns_zero() {
        let (mut storage, _buf) = make_storage(8);
        storage.seek(SeekFrom::End(0)).expect("seek to end");

        let mut read_buf: [u8; 4] = [0; 4];
        let n: usize = storage.read(&mut read_buf).expect("read should succeed");
        assert_eq!(n, 0, "reading at end should return 0");
    }

    /// Tests partial read when buffer is larger than remaining data.
    #[test]
    fn read_partial_at_boundary() {
        let (mut storage, _buf) = make_storage(8);
        storage
            .seek(SeekFrom::Start(6))
            .expect("seek should succeed");

        let mut read_buf: [u8; 8] = [0; 8];
        let n: usize = storage.read(&mut read_buf).expect("read should succeed");
        assert_eq!(n, 2, "should only read 2 remaining bytes");
    }

    // -- Write tests -------------------------------------------------------------

    /// Tests basic write and read back.
    #[test]
    fn write_and_read_back() {
        let (mut storage, _buf) = make_storage(32);
        let data: &[u8] = b"hello";
        let n: usize = storage.write(data).expect("write should succeed");
        assert_eq!(n, 5, "should write 5 bytes");

        storage
            .seek(SeekFrom::Start(0))
            .expect("seek should succeed");
        let mut read_buf: [u8; 5] = [0; 5];
        storage.read(&mut read_buf).expect("read should succeed");
        assert_eq!(&read_buf, b"hello", "read back should match written data");
    }

    /// Tests writing empty buffer.
    #[test]
    fn write_empty_buffer_returns_zero() {
        let (mut storage, _buf) = make_storage(16);
        let n: usize = storage.write(&[]).expect("write should succeed");
        assert_eq!(n, 0, "writing empty buffer should return 0");
    }

    /// Tests partial write when buffer is larger than remaining space.
    #[test]
    fn write_partial_at_boundary() {
        let (mut storage, _buf) = make_storage(8);
        storage
            .seek(SeekFrom::Start(6))
            .expect("seek should succeed");

        let data: [u8; 8] = [0xFF; 8];
        let n: usize = storage.write(&data).expect("write should succeed");
        assert_eq!(n, 2, "should only write 2 remaining bytes");
    }

    /// Tests that flush succeeds (no-op for memory).
    #[test]
    fn flush_succeeds() {
        let (mut storage, _buf) = make_storage(8);
        storage.flush().expect("flush should always succeed");
    }

    // -- Seek tests --------------------------------------------------------------

    /// Tests seek from start.
    #[test]
    fn seek_from_start() {
        let (mut storage, _buf) = make_storage(64);
        let pos: u64 = storage
            .seek(SeekFrom::Start(10))
            .expect("seek should succeed");
        assert_eq!(pos, 10, "position should be 10");
    }

    /// Tests seek from end.
    #[test]
    fn seek_from_end() {
        let (mut storage, _buf) = make_storage(64);
        let pos: u64 = storage
            .seek(SeekFrom::End(-4))
            .expect("seek should succeed");
        assert_eq!(pos, 60, "position should be 60");
    }

    /// Tests seek from current position.
    #[test]
    fn seek_from_current() {
        let (mut storage, _buf) = make_storage(64);
        storage
            .seek(SeekFrom::Start(20))
            .expect("seek should succeed");
        let pos: u64 = storage
            .seek(SeekFrom::Current(5))
            .expect("seek should succeed");
        assert_eq!(pos, 25, "position should be 25");
    }

    /// Tests seek from current with negative offset.
    #[test]
    fn seek_current_negative() {
        let (mut storage, _buf) = make_storage(64);
        storage
            .seek(SeekFrom::Start(20))
            .expect("seek should succeed");
        let pos: u64 = storage
            .seek(SeekFrom::Current(-10))
            .expect("seek should succeed");
        assert_eq!(pos, 10, "position should be 10");
    }

    /// Tests that seeking to the exact end (size) succeeds.
    #[test]
    fn seek_to_exact_end() {
        let (mut storage, _buf) = make_storage(64);
        let pos: u64 = storage
            .seek(SeekFrom::Start(64))
            .expect("seek to exact end should succeed");
        assert_eq!(pos, 64, "position should be 64");
    }

    /// Tests that seeking past the end fails.
    #[test]
    fn seek_past_end_fails() {
        let (mut storage, _buf) = make_storage(64);
        let result = storage.seek(SeekFrom::Start(65));
        assert_eq!(result.unwrap_err(), MemoryIoError::OutOfBounds, "should fail with OutOfBounds");
    }

    /// Tests that seeking to negative position fails.
    #[test]
    fn seek_negative_fails() {
        let (mut storage, _buf) = make_storage(64);
        let result = storage.seek(SeekFrom::Current(-1));
        assert_eq!(result.unwrap_err(), MemoryIoError::InvalidSeek, "should fail with InvalidSeek");
    }

    /// Tests seek to position 0 (Current(0)) returns current position.
    #[test]
    fn seek_current_zero_returns_position() {
        let (mut storage, _buf) = make_storage(64);
        storage
            .seek(SeekFrom::Start(30))
            .expect("seek should succeed");
        let pos: u64 = storage
            .seek(SeekFrom::Current(0))
            .expect("seek should succeed");
        assert_eq!(pos, 30, "Current(0) should return current position");
    }

    /// Tests that End(0) returns the size.
    #[test]
    fn seek_end_zero() {
        let (mut storage, _buf) = make_storage(64);
        let pos: u64 = storage.seek(SeekFrom::End(0)).expect("seek should succeed");
        assert_eq!(pos, 64, "End(0) should return size");
    }

    // -- Debug trait test --------------------------------------------------------

    /// Tests that Debug formatting includes field names.
    #[test]
    fn debug_format() {
        let (storage, _buf) = make_storage(32);
        let debug_str: alloc::string::String = alloc::format!("{storage:?}");
        assert!(debug_str.contains("RawMemoryStorage"), "debug output should contain type name");
        assert!(debug_str.contains("size"), "debug output should contain 'size'");
        assert!(debug_str.contains("position"), "debug output should contain 'position'");
    }
}
