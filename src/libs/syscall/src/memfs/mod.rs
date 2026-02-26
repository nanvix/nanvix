// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! In-memory filesystem interception layer for POSIX file operations.
//!
//! This module intercepts `open()`, `read()`, `write()`, `close()`, `lseek()`,
//! and `fstat()` calls to route file operations through an in-memory FAT32
//! filesystem when the target path matches a configured mount prefix. All other
//! file operations fall through to the standard linuxd IPC path.
//!
//! # Design
//!
//! - FAT32 file descriptors use a dedicated range starting at [`MEMFS_FD_BASE`]
//!   (1024) to avoid conflicting with linuxd-managed file descriptors.
//! - A static file descriptor table maps FAT32 FDs to [`fat32::File`] handles.
//! - The module is single-threaded (Nanvix guest invariant), so global state
//!   uses `UnsafeCell` with manual `Sync` implementations.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::cell::UnsafeCell;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        c_size_t,
        off_t,
    },
};

extern crate alloc;

//==================================================================================================
// Constants
//==================================================================================================

/// Base file descriptor number for in-memory filesystem handles.
const MEMFS_FD_BASE: c_int = 1024;

/// Maximum number of simultaneously open in-memory files.
const MEMFS_MAX_OPEN_FILES: usize = 64;

//==================================================================================================
// File Descriptor Table
//==================================================================================================

/// Zero-copy direct memory access for file reads.
///
/// When a file's data is stored contiguously in the FAT32 image, reads are
/// served directly from the image buffer via memcpy, bypassing all FAT32
/// cluster chain traversal.
struct DirectRead {
    /// Pointer to the file's data within the FAT32 image.
    data: *const u8,
    /// File size in bytes.
    size: usize,
    /// Current read position.
    position: usize,
}

/// An open file slot in the in-memory file descriptor table.
struct MemfsFile {
    /// The FAT32 file handle (used as fallback for non-contiguous files).
    file: Option<fat32::File>,
    /// Zero-copy fast path for contiguous files.
    direct: Option<DirectRead>,
}

/// Global file descriptor table for in-memory filesystem files.
struct MemfsFdTable {
    /// File slots indexed by (fd - MEMFS_FD_BASE).
    slots: [UnsafeCell<Option<MemfsFile>>; MEMFS_MAX_OPEN_FILES],
}

// SAFETY: Nanvix guest is single-threaded.
unsafe impl Sync for MemfsFdTable {}

impl MemfsFdTable {
    /// Creates a new empty file descriptor table.
    const fn new() -> Self {
        const NONE: UnsafeCell<Option<MemfsFile>> = UnsafeCell::new(None);
        Self {
            slots: [NONE; MEMFS_MAX_OPEN_FILES],
        }
    }

    /// Allocates a new file descriptor for the given file entry.
    fn alloc(&self, entry: MemfsFile) -> Option<c_int> {
        for i in 0..MEMFS_MAX_OPEN_FILES {
            // SAFETY: Single-threaded guest.
            let slot: &mut Option<MemfsFile> = unsafe { &mut *self.slots[i].get() };
            if slot.is_none() {
                *slot = Some(entry);
                return Some(MEMFS_FD_BASE + i as c_int);
            }
        }
        None
    }

    /// Gets a mutable reference to the file entry for a given FD.
    fn get_mut(&self, fd: c_int) -> Option<&mut MemfsFile> {
        let idx: usize = (fd - MEMFS_FD_BASE) as usize;
        if idx >= MEMFS_MAX_OPEN_FILES {
            return None;
        }
        // SAFETY: Single-threaded guest.
        let slot: &mut Option<MemfsFile> = unsafe { &mut *self.slots[idx].get() };
        slot.as_mut()
    }

    /// Closes and frees the file descriptor.
    fn close(&self, fd: c_int) -> bool {
        let idx: usize = (fd - MEMFS_FD_BASE) as usize;
        if idx >= MEMFS_MAX_OPEN_FILES {
            return false;
        }
        // SAFETY: Single-threaded guest.
        let slot: &mut Option<MemfsFile> = unsafe { &mut *self.slots[idx].get() };
        if slot.is_some() {
            *slot = None;
            true
        } else {
            false
        }
    }
}

/// Global file descriptor table.
static FD_TABLE: MemfsFdTable = MemfsFdTable::new();

/// Wrapper for a boolean flag that is safe to share across threads.
///
/// # Safety
///
/// Nanvix guest is single-threaded.
struct SyncBool(UnsafeCell<bool>);

// SAFETY: Nanvix guest is single-threaded.
unsafe impl Sync for SyncBool {}

/// Whether the in-memory filesystem has been initialized.
static INITIALIZED: SyncBool = SyncBool(UnsafeCell::new(false));

//==================================================================================================
// Initialization
//==================================================================================================

/// Returns whether the in-memory filesystem is initialized.
fn is_initialized() -> bool {
    // SAFETY: Single-threaded guest.
    unsafe { *INITIALIZED.0.get() }
}

/// Initializes the in-memory filesystem.
///
/// # Errors
///
/// Returns an error if the FAT32 library fails to initialize.
pub fn init() -> Result<(), fat32::FsError> {
    if is_initialized() {
        return Ok(());
    }
    fat32::init()?;
    // SAFETY: Single-threaded guest.
    unsafe {
        *INITIALIZED.0.get() = true;
    }
    Ok(())
}

//==================================================================================================
// Path Matching
//==================================================================================================

/// Returns `true` if the given path should be handled by the in-memory
/// filesystem (i.e., it matches a registered FAT32 mount prefix).
pub fn is_memfs_path(path: &str) -> bool {
    if !is_initialized() {
        return false;
    }
    // Check if the path or its parent resolves to a mounted FAT32 filesystem.
    if fat32::stat(path).is_ok() {
        return true;
    }
    if let Some(pos) = path.rfind('/') {
        let parent: &str = if pos == 0 { "/" } else { &path[..pos] };
        return fat32::stat(parent).is_ok();
    }
    false
}

/// Returns `true` if the given file descriptor belongs to the in-memory
/// filesystem.
pub fn is_memfs_fd(fd: c_int) -> bool {
    fd >= MEMFS_FD_BASE && fd < MEMFS_FD_BASE + MEMFS_MAX_OPEN_FILES as c_int
}

//==================================================================================================
// POSIX-Compatible Operations
//==================================================================================================

/// Opens a file in the in-memory filesystem.
///
/// If the file's data is stored contiguously in the FAT32 image, reads
/// will be served directly from memory (zero-copy). Otherwise, falls back
/// to the standard FAT32 read path.
pub fn memfs_open(path: &str, flags: c_int) -> Result<c_int, c_int> {
    let o_wronly: c_int = 1;
    let o_rdwr: c_int = 2;
    let o_creat: c_int = 0o100;
    let o_trunc: c_int = 0o1000;
    let o_excl: c_int = 0o200;

    let access_mode: c_int = flags & 3;
    let is_read_only: bool = access_mode == 0; // O_RDONLY

    // Try zero-copy direct read for read-only opens.
    if is_read_only && (flags & (o_creat | o_trunc | o_excl)) == 0 {
        if let Some((data_ptr, size)) = fat32::file_raw_region(path) {
            let entry = MemfsFile {
                file: None,
                direct: Some(DirectRead {
                    data: data_ptr,
                    size,
                    position: 0,
                }),
            };
            return FD_TABLE.alloc(entry).ok_or(-1);
        }
    }

    // Fall back to standard FAT32 open.
    let mut opts: fat32::OpenOptions = fat32::OpenOptions::new();

    if access_mode == o_wronly {
        opts = opts.write(true);
    } else if access_mode == o_rdwr {
        opts = opts.read(true).write(true);
    } else {
        opts = opts.read(true);
    }

    if flags & o_creat != 0 {
        if flags & o_excl != 0 {
            opts = opts.create_new(true);
        } else {
            opts = opts.create(true);
        }
    }

    if flags & o_trunc != 0 {
        opts = opts.truncate(true);
    }

    let file: fat32::File = opts.open(path).map_err(|_| -1)?;
    let entry = MemfsFile {
        file: Some(file),
        direct: None,
    };
    FD_TABLE.alloc(entry).ok_or(-1)
}

/// Reads from an in-memory file descriptor.
///
/// Uses zero-copy direct memory access when available, otherwise falls back
/// to the FAT32 read path.
pub fn memfs_read(fd: c_int, buf: &mut [u8]) -> Result<c_size_t, c_int> {
    let entry: &mut MemfsFile = FD_TABLE.get_mut(fd).ok_or(-1)?;

    // Fast path: direct memory read (bypasses FAT32 cluster chain).
    if let Some(ref mut direct) = entry.direct {
        let remaining: usize = direct.size.saturating_sub(direct.position);
        let to_read: usize = buf.len().min(remaining);
        if to_read == 0 {
            return Ok(0);
        }
        // SAFETY: data pointer is valid for the lifetime of the FAT32 image,
        // and position + to_read <= size (guaranteed by min above).
        unsafe {
            let src: *const u8 = direct.data.add(direct.position);
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), to_read);
        }
        direct.position += to_read;
        return Ok(to_read as c_size_t);
    }

    // Slow path: FAT32 read.
    let file: &mut fat32::File = entry.file.as_mut().ok_or(-1)?;
    let n: usize = file.read(buf).map_err(|_| -1)?;
    Ok(n as c_size_t)
}

/// Writes to an in-memory file descriptor.
pub fn memfs_write(fd: c_int, buf: &[u8]) -> Result<c_size_t, c_int> {
    let entry: &mut MemfsFile = FD_TABLE.get_mut(fd).ok_or(-1)?;
    let file: &mut fat32::File = entry.file.as_mut().ok_or(-1)?;
    let n: usize = file.write(buf).map_err(|_| -1)?;
    Ok(n as c_size_t)
}

/// Seeks an in-memory file descriptor.
pub fn memfs_lseek(fd: c_int, offset: off_t, whence: c_int) -> Result<off_t, c_int> {
    let entry: &mut MemfsFile = FD_TABLE.get_mut(fd).ok_or(-1)?;

    // Fast path: direct seek (O(1) position update).
    if let Some(ref mut direct) = entry.direct {
        let new_pos: i64 = match whence {
            0 => offset,                          // SEEK_SET
            1 => direct.position as i64 + offset, // SEEK_CUR
            2 => direct.size as i64 + offset,     // SEEK_END
            _ => return Err(-1),
        };
        if new_pos < 0 || new_pos > direct.size as i64 {
            return Err(-1);
        }
        direct.position = new_pos as usize;
        return Ok(new_pos as off_t);
    }

    // Slow path: FAT32 seek.
    let file: &mut fat32::File = entry.file.as_mut().ok_or(-1)?;
    let pos: u64 = file.seek(whence, offset).map_err(|_| -1)?;
    Ok(pos as off_t)
}

/// Gets file status for an in-memory file descriptor.
pub fn memfs_fstat(fd: c_int, buf: &mut ::sysapi::sys_stat::stat) -> Result<(), c_int> {
    let entry: &mut MemfsFile = FD_TABLE.get_mut(fd).ok_or(-1)?;

    // Get file size from direct read info or FAT32 file handle.
    let size: u64 = if let Some(ref direct) = entry.direct {
        direct.size as u64
    } else {
        let file: &mut fat32::File = entry.file.as_mut().ok_or(-1)?;
        file.size().map_err(|_| -1)?
    };

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }

    buf.st_size = size as off_t;
    buf.st_mode = 0o100444; // Regular file, read-only.
    buf.st_blksize = 4096;
    buf.st_blocks = size.div_ceil(512) as off_t;

    Ok(())
}

/// Closes an in-memory file descriptor.
pub fn memfs_close(fd: c_int) -> Result<(), c_int> {
    if FD_TABLE.close(fd) {
        Ok(())
    } else {
        Err(-1)
    }
}

//==================================================================================================
// C-Compatible Initialization API
//==================================================================================================

/// Encoded 8-byte "RAMFS   " tag exposed by the MicroVM RAMFS MMIO region.
const RAMFS_MMIO_TAG: u64 = u64::from_be_bytes(*b"RAMFS   ");

/// Initializes the in-memory filesystem from the RAMFS MMIO region.
///
/// # Parameters
///
/// - `mount_path`: C string with the mount path (e.g., "/model").
///
/// # Returns
///
/// 0 on success, -1 on failure.
///
/// # Safety
///
/// `mount_path` must be a valid null-terminated C string.
pub unsafe fn memfs_init_from_ramfs(mount_path: *const ::sysapi::ffi::c_char) -> c_int {
    let path: &str = match ::core::ffi::CStr::from_ptr(mount_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match init_from_ramfs_inner(path) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Inner implementation for RAMFS-based initialization.
fn init_from_ramfs_inner(mount_path: &str) -> Result<(), c_int> {
    use ::sys::{
        mm::Address,
        pm::Capability,
    };

    // Initialize the FAT32 filesystem.
    init().map_err(|_| -1)?;

    // Acquire IO management capability.
    ::sys::kcall::pm::capctl(Capability::IoManagement, true).map_err(|_| -1)?;

    let result: Result<(), c_int> = (|| {
        ::sys::kcall::mm::mmio_alloc(RAMFS_MMIO_TAG).map_err(|_| -1)?;

        let info: ::sys::mm::MmioRegionInfo =
            ::sys::kcall::mm::mmio_info(RAMFS_MMIO_TAG).map_err(|_| -1)?;
        let total_size: usize = info.size();

        // Mount the FAT image directly from the MMIO region (zero-copy).
        // The RAMFS is read-only, but the FAT32 library's storage interface
        // requires *mut u8. Since we never write to it, the cast is safe.
        let base_ptr: *mut u8 = info.base().as_ptr() as *mut u8;
        unsafe {
            fat32::mount(mount_path, base_ptr, total_size).map_err(|_| -1)?;
        }

        // Release IO management capability but keep the MMIO mapping alive.
        ::sys::kcall::pm::capctl(Capability::IoManagement, false).map_err(|_| -1)?;

        Ok(())
    })();

    result
}

/// Returns the size of the file at the given path in the in-memory filesystem.
///
/// # Safety
///
/// `path` must be a valid null-terminated C string.
pub unsafe fn memfs_file_size(path: *const ::sysapi::ffi::c_char) -> i64 {
    let path_str: &str = match ::core::ffi::CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match fat32::stat(path_str) {
        Ok(info) => info.size as i64,
        Err(_) => -1,
    }
}
