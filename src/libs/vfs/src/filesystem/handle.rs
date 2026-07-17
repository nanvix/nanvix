// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Open FAT filesystem file handle.

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
// Structures
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

//==================================================================================================
// Implementations
//==================================================================================================

impl File {
    /// Creates a file handle and associates it with its mount point.
    pub(super) fn new(inner: FatFile<'static>, mount_path: String) -> Self {
        Self { inner, mount_path }
    }

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
