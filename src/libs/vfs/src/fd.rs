// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! VFS file descriptor table and POSIX-compatible FD operations.
//!
//! This module provides the system-wide file descriptor table that maps
//! integer FDs to backend-specific file handles, and POSIX-compatible
//! operations (`open`, `read`, `write`, `lseek`, `fstat`, `close`, `stat`)
//! that route through the FD table.
//!
//! # File Handle Abstraction
//!
//! [`VfsFileHandle`] is an enum that dispatches to concrete filesystem
//! backends. To add a new backend:
//! 1. Add a variant to [`VfsFileHandle`].
//! 2. Implement `read`, `write`, `seek`, and `size` for the new variant.
//! 3. Update [`crate::fat32_backend`] (or create a new backend module).

//==================================================================================================
// Imports
//==================================================================================================

use crate::fat32_backend;
use ::fat32::Fat32Error;
use ::spin::Mutex;
use ::sysapi::{
    fcntl::file_control_request,
    ffi::c_int,
    sys_stat::{
        file_mode,
        file_type,
    },
    sys_types::{
        c_size_t,
        off_t,
    },
    unistd::file_seek,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Base file descriptor number for VFS-managed handles.
///
/// VFS file descriptors occupy the range `[VFS_FD_BASE, VFS_FD_BASE + VFS_MAX_OPEN_FILES)`.
/// This range must not overlap with linuxd-assigned file descriptors.
const VFS_FD_BASE: c_int = 1024;

/// Maximum number of simultaneously open VFS files.
const VFS_MAX_OPEN_FILES: usize = 64;

/// Block size reported in stat results (bytes).
const STAT_BLOCK_SIZE: i64 = 4096;

/// Sector size used for `st_blocks` computation (POSIX convention: 512 bytes).
const STAT_SECTOR_SIZE: u64 = 512;

//==================================================================================================
// Metadata
//==================================================================================================

/// File metadata returned by stat operations.
///
/// This is the VFS-level metadata type, independent of any concrete
/// filesystem. Backend modules translate their native metadata into this
/// type.
pub struct VfsStat {
    /// File size in bytes (0 for directories).
    size: u64,
    /// Whether this entry is a directory.
    is_dir: bool,
}

impl VfsStat {
    /// Creates a new `VfsStat`.
    pub fn new(size: u64, is_dir: bool) -> Self {
        Self { size, is_dir }
    }

    /// Returns the file size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns whether this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
}

//==================================================================================================
// Direct Read Handle
//==================================================================================================

/// Zero-copy direct memory access handle for file reads.
///
/// When a file's data is stored contiguously in an in-memory filesystem
/// image, reads can be served directly from the image buffer via memcpy,
/// bypassing all cluster chain traversal.
pub struct DirectReadHandle {
    /// Pointer to the file's data within the filesystem image.
    data: *const u8,
    /// File size in bytes.
    size: usize,
    /// Current read position.
    position: usize,
}

impl DirectReadHandle {
    /// Creates a new direct read handle.
    pub fn new(data: *const u8, size: usize) -> Self {
        Self {
            data,
            size,
            position: 0,
        }
    }

    /// Reads data from the direct memory region.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let remaining: usize = self.size.saturating_sub(self.position);
        let to_read: usize = buf.len().min(remaining);
        if to_read == 0 {
            return 0;
        }
        // SAFETY: data pointer is valid for the lifetime of the filesystem
        // image, and position + to_read <= size (guaranteed by min above).
        unsafe {
            let src: *const u8 = self.data.add(self.position);
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), to_read);
        }
        self.position += to_read;
        to_read
    }

    /// Seeks to a position in the direct memory region.
    pub fn seek(&mut self, offset: off_t, whence: c_int) -> Result<off_t, Fat32Error> {
        let new_pos: i64 = match whence {
            file_seek::SEEK_SET => offset,
            file_seek::SEEK_CUR => self.position as i64 + offset,
            file_seek::SEEK_END => self.size as i64 + offset,
            _ => return Err(Fat32Error::InvalidArgument),
        };
        if new_pos < 0 || new_pos > self.size as i64 {
            return Err(Fat32Error::InvalidSeek);
        }
        self.position = new_pos as usize;
        Ok(new_pos as off_t)
    }

    /// Returns the file size.
    pub fn size(&self) -> usize {
        self.size
    }
}

//==================================================================================================
// VFS File Handle
//==================================================================================================

/// An open file handle managed by the VFS.
///
/// Each variant corresponds to a concrete filesystem backend or an
/// optimization path. The VFS FD table stores these handles and
/// dispatches operations to the appropriate variant.
pub enum VfsFileHandle {
    /// File opened through the FAT32 backend.
    Fat32(crate::File),
    /// Zero-copy direct memory read (contiguous file optimization).
    DirectRead(DirectReadHandle),
}

impl VfsFileHandle {
    /// Reads data from the file.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.read(buf),
            VfsFileHandle::DirectRead(handle) => Ok(handle.read(buf)),
        }
    }

    /// Writes data to the file.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.write(buf),
            VfsFileHandle::DirectRead(_) => Err(Fat32Error::ReadOnly),
        }
    }

    /// Seeks to a position in the file.
    pub fn seek(&mut self, offset: off_t, whence: c_int) -> Result<off_t, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => {
                let pos: u64 = file.seek(whence, offset)?;
                Ok(pos as off_t)
            },
            VfsFileHandle::DirectRead(handle) => handle.seek(offset, whence),
        }
    }

    /// Returns the file size in bytes.
    pub fn size(&mut self) -> Result<u64, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.size(),
            VfsFileHandle::DirectRead(handle) => Ok(handle.size() as u64),
        }
    }
}

//==================================================================================================
// File Descriptor Table
//==================================================================================================

/// An open file slot in the VFS file descriptor table.
struct VfsEntry {
    /// The file handle from any backend.
    handle: VfsFileHandle,
}

// SAFETY: VfsEntry contains FAT filesystem types that use `Cell` internally
// (e.g., `FsStatusFlags`), which prevents auto-impl of `Send`. This is safe
// because all access to VfsEntry goes through a `spin::Mutex`, ensuring
// exclusive access. The Cell is never shared across threads without the mutex.
unsafe impl Send for VfsEntry {}

/// Global file descriptor table for VFS-managed files.
///
/// Each slot is individually protected by a [`spin::Mutex`] so that
/// concurrent operations on different FDs do not block each other.
struct VfsFdTable {
    /// File slots indexed by (fd - VFS_FD_BASE).
    slots: [Mutex<Option<VfsEntry>>; VFS_MAX_OPEN_FILES],
}

impl VfsFdTable {
    /// Creates a new empty file descriptor table.
    #[allow(clippy::declare_interior_mutable_const)]
    const fn new() -> Self {
        const NONE: Mutex<Option<VfsEntry>> = Mutex::new(None);
        Self {
            slots: [NONE; VFS_MAX_OPEN_FILES],
        }
    }

    /// Allocates a new file descriptor for the given file handle.
    fn alloc(&self, handle: VfsFileHandle) -> Result<c_int, Fat32Error> {
        for i in 0..VFS_MAX_OPEN_FILES {
            let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = self.slots[i].lock();
            if slot.is_none() {
                *slot = Some(VfsEntry { handle });
                return Ok(VFS_FD_BASE + i as c_int);
            }
        }
        Err(Fat32Error::TooManyOpenFiles)
    }

    /// Locks the slot for a given FD and returns the guard.
    ///
    /// Returns `Err(Fat32Error::InvalidFd)` if the FD is out of range.
    fn lock(&self, fd: c_int) -> Result<spin::MutexGuard<'_, Option<VfsEntry>>, Fat32Error> {
        let idx: usize = (fd - VFS_FD_BASE) as usize;
        if idx >= VFS_MAX_OPEN_FILES {
            return Err(Fat32Error::InvalidFd);
        }
        Ok(self.slots[idx].lock())
    }

    /// Closes and frees the file descriptor.
    fn close(&self, fd: c_int) -> Result<(), Fat32Error> {
        let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = self.lock(fd)?;
        if slot.is_some() {
            *slot = None;
            Ok(())
        } else {
            Err(Fat32Error::InvalidFd)
        }
    }
}

/// Global file descriptor table.
static FD_TABLE: VfsFdTable = VfsFdTable::new();

//==================================================================================================
// Path Routing
//==================================================================================================

/// Returns `true` if the given path is handled by the VFS.
pub fn is_vfs_path(path: &str) -> bool {
    fat32_backend::exists(path)
}

/// Returns `true` if the given file descriptor belongs to the VFS.
pub fn is_vfs_fd(fd: c_int) -> bool {
    fd >= VFS_FD_BASE && fd < VFS_FD_BASE + VFS_MAX_OPEN_FILES as c_int
}

//==================================================================================================
// POSIX-Compatible Operations
//==================================================================================================

/// Opens a file through the VFS and allocates a system-wide FD.
pub fn vfs_open(path: &str, flags: c_int) -> Result<c_int, Fat32Error> {
    let handle: VfsFileHandle = fat32_backend::open(path, flags)?;
    FD_TABLE.alloc(handle)
}

/// Reads from a VFS file descriptor.
pub fn vfs_read(fd: c_int, buf: &mut [u8]) -> Result<c_size_t, Fat32Error> {
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;
    let n: usize = entry.handle.read(buf)?;
    Ok(n as c_size_t)
}

/// Writes to a VFS file descriptor.
pub fn vfs_write(fd: c_int, buf: &[u8]) -> Result<c_size_t, Fat32Error> {
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;
    let n: usize = entry.handle.write(buf)?;
    Ok(n as c_size_t)
}

/// Seeks a VFS file descriptor.
pub fn vfs_lseek(fd: c_int, offset: off_t, whence: c_int) -> Result<off_t, Fat32Error> {
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;
    entry.handle.seek(offset, whence)
}

/// Gets file status for a VFS file descriptor.
pub fn vfs_fstat(fd: c_int, buf: &mut ::sysapi::sys_stat::stat) -> Result<(), Fat32Error> {
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;
    let size: u64 = entry.handle.size()?;

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }

    buf.st_size = size as off_t;
    buf.st_mode = file_type::S_IFREG | file_mode::S_IRUSR | file_mode::S_IRGRP | file_mode::S_IROTH;
    buf.st_blksize = STAT_BLOCK_SIZE;
    buf.st_blocks = size.div_ceil(STAT_SECTOR_SIZE) as off_t;

    Ok(())
}

/// Closes a VFS file descriptor.
pub fn vfs_close(fd: c_int) -> Result<(), Fat32Error> {
    FD_TABLE.close(fd)
}

/// Gets file status for a path through the VFS.
pub fn vfs_stat(path: &str, buf: &mut ::sysapi::sys_stat::stat) -> Result<(), Fat32Error> {
    let info: VfsStat = fat32_backend::stat(path)?;

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }

    buf.st_size = info.size() as off_t;
    buf.st_mode = if info.is_dir() {
        file_type::S_IFDIR
            | file_mode::S_IRWXU
            | file_mode::S_IRGRP
            | file_mode::S_IXGRP
            | file_mode::S_IROTH
            | file_mode::S_IXOTH
    } else {
        file_type::S_IFREG | file_mode::S_IRUSR | file_mode::S_IRGRP | file_mode::S_IROTH
    };
    buf.st_blksize = STAT_BLOCK_SIZE;
    buf.st_blocks = info.size().div_ceil(STAT_SECTOR_SIZE) as off_t;

    Ok(())
}

/// Renames a file or directory through the VFS.
///
/// Both paths must be on the same VFS mount.
pub fn vfs_rename(old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
    crate::rename(old_path, new_path)
}

/// Deletes a file through the VFS.
pub fn vfs_unlink(path: &str) -> Result<(), Fat32Error> {
    crate::unlink(path)
}

/// Creates a directory through the VFS.
pub fn vfs_mkdir(path: &str) -> Result<(), Fat32Error> {
    crate::mkdir(path)
}

/// Removes an empty directory through the VFS.
pub fn vfs_rmdir(path: &str) -> Result<(), Fat32Error> {
    crate::rmdir(path)
}

/// Changes the VFS current working directory.
pub fn vfs_chdir(path: &str) -> Result<(), Fat32Error> {
    crate::chdir(path)
}

/// Gets the VFS current working directory.
pub fn vfs_getcwd() -> Result<alloc::string::String, Fat32Error> {
    crate::cwd()
}

/// Lists directory contents through the VFS.
///
/// Returns a vector of directory entries.
pub fn vfs_readdir(path: &str) -> Result<alloc::vec::Vec<crate::DirEntry>, Fat32Error> {
    crate::read_dir(path)
}

/// Truncates a VFS file descriptor to the given length.
///
/// POSIX requires that `ftruncate()` does not change the file offset.
/// The current offset is saved before truncation and restored afterwards.
pub fn vfs_ftruncate(fd: c_int, length: off_t) -> Result<(), Fat32Error> {
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;
    match &mut entry.handle {
        VfsFileHandle::Fat32(file) => {
            // Save the current offset so we can restore it after truncation.
            let saved: u64 = file.seek(file_seek::SEEK_CUR, 0)?;
            let result: Result<(), Fat32Error> = (|| {
                file.seek(file_seek::SEEK_SET, length)?;
                file.truncate()?;
                Ok(())
            })();
            // Restore the original offset regardless of success or failure.
            let _ = file.seek(file_seek::SEEK_SET, saved as off_t);
            result
        },
        VfsFileHandle::DirectRead(_) => Err(Fat32Error::ReadOnly),
    }
}

/// Syncs a VFS file descriptor (flush buffered data).
///
/// For in-memory FAT, this flushes the fatfs buffers.
pub fn vfs_fsync(fd: c_int) -> Result<(), Fat32Error> {
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;
    match &mut entry.handle {
        VfsFileHandle::Fat32(file) => file.flush(),
        VfsFileHandle::DirectRead(_) => Ok(()), // Nothing to sync for read-only.
    }
}

/// Checks if a VFS file descriptor refers to a terminal.
///
/// VFS file descriptors are never terminals.
pub fn vfs_isatty(_fd: c_int) -> bool {
    false
}

/// Reads from a VFS file descriptor at a given offset without changing position.
pub fn vfs_pread(fd: c_int, buf: &mut [u8], offset: off_t) -> Result<c_size_t, Fat32Error> {
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;

    // Save current position, seek to offset, read, then restore.
    let saved: off_t = entry.handle.seek(0, file_seek::SEEK_CUR)?;
    entry.handle.seek(offset, file_seek::SEEK_SET)?;
    let result: Result<usize, Fat32Error> = entry.handle.read(buf);
    // Always restore position, even if read failed.
    let _ = entry.handle.seek(saved, file_seek::SEEK_SET);
    let n: usize = result?;
    Ok(n as c_size_t)
}

/// Writes to a VFS file descriptor at a given offset without changing position.
pub fn vfs_pwrite(fd: c_int, buf: &[u8], offset: off_t) -> Result<c_size_t, Fat32Error> {
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;

    // Save current position, seek to offset, write, then restore.
    let saved: off_t = entry.handle.seek(0, file_seek::SEEK_CUR)?;
    entry.handle.seek(offset, file_seek::SEEK_SET)?;
    let result: Result<usize, Fat32Error> = entry.handle.write(buf);
    // Always restore position, even if write failed.
    let _ = entry.handle.seek(saved, file_seek::SEEK_SET);
    let n: usize = result?;
    Ok(n as c_size_t)
}

/// Checks file accessibility through the VFS.
///
/// Returns `Ok(())` if the path exists, `Err` otherwise.
/// FAT32 does not have UNIX permissions, so only existence is checked.
pub fn vfs_access(path: &str) -> Result<(), Fat32Error> {
    crate::stat(path).map(|_| ())
}

/// File control operation on a VFS file descriptor.
///
/// Only `F_GETFL` and `F_SETFL` are supported (as no-ops for FAT32).
/// Other commands return `NotSupported`.
pub fn vfs_fcntl(fd: c_int, cmd: c_int) -> Result<c_int, Fat32Error> {
    // Verify the fd is valid.
    let mut slot: spin::MutexGuard<'_, Option<VfsEntry>> = FD_TABLE.lock(fd)?;
    let _entry: &mut VfsEntry = slot.as_mut().ok_or(Fat32Error::InvalidFd)?;

    match cmd {
        file_control_request::F_GETFL => Ok(0), // No meaningful flags for FAT32.
        file_control_request::F_SETFL => Ok(0), // Accept but ignore (no O_NONBLOCK etc.).
        _ => Err(Fat32Error::NotSupported),      // Other commands not supported.
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // -- VfsStat tests -----------------------------------------------------------

    /// Tests VfsStat construction and accessors for a file.
    #[test]
    fn vfs_stat_file() {
        let s: VfsStat = VfsStat::new(1024, false);
        assert_eq!(s.size(), 1024, "file size should be 1024");
        assert!(!s.is_dir(), "should not be a directory");
    }

    /// Tests VfsStat construction and accessors for a directory.
    #[test]
    fn vfs_stat_directory() {
        let s: VfsStat = VfsStat::new(0, true);
        assert_eq!(s.size(), 0, "directory size should be 0");
        assert!(s.is_dir(), "should be a directory");
    }

    // -- DirectReadHandle tests --------------------------------------------------

    /// Tests reading from a direct read handle.
    #[test]
    fn direct_read_basic() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
        let mut buf: [u8; 3] = [0; 3];

        let n: usize = handle.read(&mut buf);
        assert_eq!(n, 3, "should read 3 bytes");
        assert_eq!(buf, [1, 2, 3], "first 3 bytes");
    }

    /// Tests reading until EOF.
    #[test]
    fn direct_read_to_eof() {
        let data: [u8; 3] = [10, 20, 30];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
        let mut buf: [u8; 10] = [0; 10];

        let n: usize = handle.read(&mut buf);
        assert_eq!(n, 3, "should read all 3 bytes");
        assert_eq!(&buf[..3], &[10, 20, 30]);

        let n2: usize = handle.read(&mut buf);
        assert_eq!(n2, 0, "should return 0 at EOF");
    }

    /// Tests reading with empty buffer.
    #[test]
    fn direct_read_empty_buffer() {
        let data: [u8; 3] = [1, 2, 3];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
        let mut buf: [u8; 0] = [];

        let n: usize = handle.read(&mut buf);
        assert_eq!(n, 0, "empty buffer should read 0 bytes");
    }

    /// Tests SEEK_SET on a direct read handle.
    #[test]
    fn direct_seek_set() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        // Read 2 bytes to advance position.
        let mut buf: [u8; 2] = [0; 2];
        handle.read(&mut buf);

        // Seek back to start.
        let pos: off_t = handle
            .seek(0, file_seek::SEEK_SET)
            .expect("SEEK_SET should succeed");
        assert_eq!(pos, 0, "position should be 0");

        // Read again.
        let n: usize = handle.read(&mut buf);
        assert_eq!(n, 2, "should read 2 bytes after seek");
        assert_eq!(buf, [1, 2], "should re-read first 2 bytes");
    }

    /// Tests SEEK_CUR on a direct read handle.
    #[test]
    fn direct_seek_cur() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        // Read 2 bytes to advance position.
        let mut buf: [u8; 2] = [0; 2];
        handle.read(&mut buf);

        // Seek forward by 1 (relative).
        let pos: off_t = handle
            .seek(1, file_seek::SEEK_CUR)
            .expect("SEEK_CUR should succeed");
        assert_eq!(pos, 3, "position should be 3");

        // Read next byte.
        let mut one: [u8; 1] = [0];
        let n: usize = handle.read(&mut one);
        assert_eq!(n, 1);
        assert_eq!(one[0], 4, "should be the 4th byte");
    }

    /// Tests SEEK_END on a direct read handle.
    #[test]
    fn direct_seek_end() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        let pos: off_t = handle
            .seek(0, file_seek::SEEK_END)
            .expect("SEEK_END should succeed");
        assert_eq!(pos, 5, "position should be at end");

        let pos2: off_t = handle
            .seek(-2, file_seek::SEEK_END)
            .expect("SEEK_END(-2) should succeed");
        assert_eq!(pos2, 3, "position should be 3");
    }

    /// Tests that seeking to a negative position fails.
    #[test]
    fn direct_seek_negative_fails() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        let result = handle.seek(-1, file_seek::SEEK_SET);
        assert!(result.is_err(), "negative SEEK_SET should fail");
    }

    /// Tests that seeking past end fails.
    #[test]
    fn direct_seek_past_end_fails() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        let result = handle.seek(6, file_seek::SEEK_SET);
        assert!(result.is_err(), "seeking past end should fail");
    }

    /// Tests that an invalid whence value fails.
    #[test]
    fn direct_seek_invalid_whence() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        let result = handle.seek(0, 99);
        assert!(result.is_err(), "invalid whence should fail");
    }

    /// Tests the size accessor.
    #[test]
    fn direct_read_size() {
        let data: [u8; 42] = [0; 42];
        let handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
        assert_eq!(handle.size(), 42, "size should match data length");
    }

    // -- VfsFileHandle::DirectRead dispatch tests --------------------------------

    /// Tests VfsFileHandle::DirectRead read dispatch.
    #[test]
    fn vfs_handle_direct_read() {
        let data: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
        let mut handle: VfsFileHandle =
            VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

        let mut buf: [u8; 4] = [0; 4];
        let n: usize = handle.read(&mut buf).expect("read should succeed");
        assert_eq!(n, 4);
        assert_eq!(buf, data);
    }

    /// Tests VfsFileHandle::DirectRead write fails (read-only).
    #[test]
    fn vfs_handle_direct_write_fails() {
        let data: [u8; 4] = [0; 4];
        let mut handle: VfsFileHandle =
            VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

        let result = handle.write(&[1, 2, 3]);
        assert!(result.is_err(), "writing to DirectRead should fail");
    }

    /// Tests VfsFileHandle::DirectRead seek dispatch.
    #[test]
    fn vfs_handle_direct_seek() {
        let data: [u8; 10] = [0; 10];
        let mut handle: VfsFileHandle =
            VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

        let pos: off_t = handle
            .seek(5, file_seek::SEEK_SET)
            .expect("seek should succeed");
        assert_eq!(pos, 5);
    }

    /// Tests VfsFileHandle::DirectRead size dispatch.
    #[test]
    fn vfs_handle_direct_size() {
        let data: [u8; 100] = [0; 100];
        let mut handle: VfsFileHandle =
            VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

        let size: u64 = handle.size().expect("size should succeed");
        assert_eq!(size, 100);
    }

    // -- FD range tests ----------------------------------------------------------

    /// Tests that VFS FD base is outside linuxd range.
    #[test]
    fn vfs_fd_base_is_high() {
        assert!(VFS_FD_BASE >= 1024, "VFS FD base should be >= 1024 to avoid linuxd conflicts");
    }

    /// Tests is_vfs_fd with FDs in range.
    #[test]
    fn is_vfs_fd_in_range() {
        assert!(is_vfs_fd(VFS_FD_BASE), "base FD should be a VFS FD");
        assert!(
            is_vfs_fd(VFS_FD_BASE + VFS_MAX_OPEN_FILES as c_int - 1),
            "last FD should be a VFS FD"
        );
    }

    /// Tests is_vfs_fd with FDs out of range.
    #[test]
    fn is_vfs_fd_out_of_range() {
        assert!(!is_vfs_fd(0), "FD 0 should not be a VFS FD");
        assert!(!is_vfs_fd(VFS_FD_BASE - 1), "FD below base should not be a VFS FD");
        assert!(
            !is_vfs_fd(VFS_FD_BASE + VFS_MAX_OPEN_FILES as c_int),
            "FD past max should not be a VFS FD"
        );
    }
}
