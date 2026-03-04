// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Global filesystem state management.
//!
//! This module manages the global VFS state and provides mount/unmount
//! operations for FAT filesystems. The guest application must call [`init()`]
//! before using any filesystem operations, then use [`mount_image()`] or
//! [`create_mount()`] to add FAT filesystems.
//!
//! # Thread Safety
//!
//! All global state is protected by [`spin::Mutex`] to ensure safe concurrent
//! access. Nanvix guests may create kernel threads, so the filesystem state
//! must be properly synchronized.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mount::{
    Mount,
    Vfs,
};
use ::alloc::{
    boxed::Box,
    string::String,
    vec::Vec,
};
use ::fat32::{
    Fat,
    Fat32Error,
    RawMemoryStorage,
};
use ::spin::Mutex;

//==================================================================================================
// Constants
//==================================================================================================

/// Minimum FAT image size (64KB for FAT12).
pub const MIN_FAT_SIZE: usize = 64 * 1024;

/// Maximum FAT image size (128MB to prevent excessive memory use).
pub const MAX_FAT_SIZE: usize = 128 * 1024 * 1024;

//==================================================================================================
// Global State
//==================================================================================================

/// Global VFS state, protected by a spin mutex.
static VFS_STATE: Mutex<Option<Vfs>> = Mutex::new(None);

/// Tracks guest-created mounts for unmount permission checks and
/// memory deallocation. Protected by a spin mutex.
static GUEST_MOUNTS: Mutex<Vec<GuestMountInfo>> = Mutex::new(Vec::new());

/// Tracks the number of open files per mount path. Protected by a spin mutex.
static OPEN_FILE_COUNTS: Mutex<Vec<OpenFileCount>> = Mutex::new(Vec::new());

//==================================================================================================
// Supporting Types
//==================================================================================================

/// Information about a guest-created mount.
struct GuestMountInfo {
    /// Mount path (e.g., "/scratch").
    path: String,
    /// Pointer to the allocated memory (for deallocation).
    memory_ptr: *mut u8,
    /// Size of the allocated memory.
    memory_size: usize,
}

// SAFETY: GuestMountInfo is only accessed through the GUEST_MOUNTS Mutex,
// which ensures exclusive access. The raw pointer represents owned heap
// memory that is only accessed by this module.
unsafe impl Send for GuestMountInfo {}

/// Tracks the number of open files for a mount.
struct OpenFileCount {
    /// Mount path that this counter is associated with.
    mount_path: String,
    /// Number of currently open file handles on this mount.
    count: usize,
}

/// Scope guard that frees a heap allocation on drop unless disarmed.
///
/// Used in [`create_mount()`] to ensure the allocated FAT image memory
/// is freed if any step after allocation fails, without duplicating
/// the cleanup logic at every error site.
struct MemoryGuard {
    /// Pointer to the allocated memory.
    ptr: *mut u8,
    /// Size of the allocated memory.
    size: usize,
    /// Whether the guard is still armed (will free on drop).
    armed: bool,
}

impl MemoryGuard {
    /// Creates a new armed guard.
    fn new(ptr: *mut u8, size: usize) -> Self {
        Self {
            ptr,
            size,
            armed: true,
        }
    }

    /// Disarms the guard so the memory will not be freed on drop.
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for MemoryGuard {
    fn drop(&mut self) {
        if self.armed {
            // SAFETY: ptr was created from Box::into_raw in create_mount().
            unsafe {
                let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(self.ptr, self.size));
            }
        }
    }
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Initializes the filesystem with an empty VFS.
///
/// Must be called before any other filesystem operations.
///
/// # Errors
///
/// Returns [`Fat32Error::NotSupported`] if the filesystem is already
/// initialized.
pub fn init() -> Result<(), Fat32Error> {
    let mut state = VFS_STATE.lock();
    if state.is_some() {
        return Err(Fat32Error::NotSupported);
    }

    *state = Some(Vfs::new());
    Ok(())
}

/// Returns true if the filesystem is initialized.
#[must_use]
pub fn is_initialized() -> bool {
    VFS_STATE.lock().is_some()
}

/// Mounts an existing FAT image from a memory region.
///
/// # Parameters
///
/// - `mount_path`: Absolute path where the mount will be accessible
///   (e.g., "/data"). Must start with "/".
/// - `ptr`: Pointer to the FAT image in memory.
/// - `size`: Size of the memory region in bytes.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if `init()` has not been called.
/// - [`Fat32Error::InvalidPath`] if `mount_path` doesn't start with "/".
/// - [`Fat32Error::InvalidArgument`] if `ptr` is null or `size` is zero.
/// - [`Fat32Error::AlreadyExists`] if a mount already exists at this path.
/// - [`Fat32Error::IoError`] if the FAT image is invalid or corrupted.
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` points to valid memory containing a FAT filesystem image.
/// - The memory remains valid for the lifetime of the mount.
/// - The memory region is at least `size` bytes.
pub unsafe fn mount_image(mount_path: &str, ptr: *mut u8, size: usize) -> Result<(), Fat32Error> {
    if !mount_path.starts_with('/') {
        return Err(Fat32Error::InvalidPath);
    }

    // SAFETY: Caller guarantees memory region validity.
    let fat: Fat = unsafe { Fat::from_memory(ptr, size)? };
    let mount: Mount = Mount::new(String::from(mount_path), fat)?;

    let mut state = VFS_STATE.lock();
    let vfs: &mut Vfs = state.as_mut().ok_or(Fat32Error::NotInitialized)?;
    vfs.add_mount(mount)?;

    Ok(())
}

/// Creates a new in-memory FAT filesystem and mounts it.
///
/// Allocates memory from the guest heap, formats it as FAT, and registers
/// it in the VFS at the given mount path.
///
/// # Parameters
///
/// - `mount_path`: Absolute path where the mount will be accessible.
///   Must start with "/" and not conflict with existing mounts.
/// - `size`: Size in bytes for the FAT image. Must be between
///   [`MIN_FAT_SIZE`] and [`MAX_FAT_SIZE`].
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if `init()` has not been called.
/// - [`Fat32Error::InvalidPath`] if `mount_path` doesn't start with "/".
/// - [`Fat32Error::InvalidArgument`] if `size` is out of range.
/// - [`Fat32Error::AlreadyExists`] if a mount already exists at this path.
/// - [`Fat32Error::IoError`] if formatting fails.
pub fn create_mount(mount_path: &str, size: usize) -> Result<(), Fat32Error> {
    if !mount_path.starts_with('/') {
        return Err(Fat32Error::InvalidPath);
    }

    if size < MIN_FAT_SIZE {
        return Err(Fat32Error::InvalidArgument);
    }
    if size > MAX_FAT_SIZE {
        return Err(Fat32Error::InvalidArgument);
    }

    // Check VFS is initialized (brief lock).
    {
        let state = VFS_STATE.lock();
        if state.is_none() {
            return Err(Fat32Error::NotInitialized);
        }
    }

    // Allocate memory for the FAT image (no lock held — allocation may be slow).
    let memory: Box<[u8]> = alloc::vec![0u8; size].into_boxed_slice();
    let memory_ptr: *mut u8 = Box::into_raw(memory) as *mut u8;

    // Guard ensures the heap allocation is freed on any error path.
    let mut guard: MemoryGuard = MemoryGuard::new(memory_ptr, size);

    // Format the memory as FAT.
    // SAFETY: memory_ptr points to valid, zeroed memory of `size` bytes.
    unsafe { format_fat_in_memory(memory_ptr, size)? };

    // Create Fat from the formatted memory.
    // SAFETY: memory_ptr points to valid FAT image of `size` bytes.
    let fat: Fat = unsafe { Fat::from_memory(memory_ptr, size)? };

    let mount: Mount = Mount::new(String::from(mount_path), fat)?;

    // Add to VFS (lock held).
    {
        let mut state = VFS_STATE.lock();
        let vfs: &mut Vfs = state.as_mut().ok_or(Fat32Error::NotInitialized)?;
        vfs.add_mount(mount)?;
    }

    // Mount succeeded — disarm the guard so memory is not freed.
    guard.disarm();

    // Track this as a guest-created mount (separate lock).
    GUEST_MOUNTS.lock().push(GuestMountInfo {
        path: String::from(mount_path),
        memory_ptr,
        memory_size: size,
    });

    Ok(())
}

/// Unmounts a guest-created FAT mount and frees its memory.
///
/// Only mounts created via [`create_mount()`] can be unmounted.
/// Attempting to unmount a mount created via [`mount_image()`] will fail with
/// [`Fat32Error::PermissionDenied`].
///
/// # Parameters
///
/// - `mount_path`: The path of the mount to remove.
///
/// # Errors
///
/// - [`Fat32Error::NotInitialized`] if `init()` has not been called.
/// - [`Fat32Error::NotFound`] if no mount exists at this path.
/// - [`Fat32Error::PermissionDenied`] if mount was not created by
///   [`create_mount()`].
/// - [`Fat32Error::FileLocked`] if files are still open on this mount.
pub fn unmount(mount_path: &str) -> Result<(), Fat32Error> {
    // Lock ordering: each step acquires and releases a single lock before
    // proceeding to the next. The sequence is:
    //   1. GUEST_MOUNTS  (check ownership)
    //   2. OPEN_FILE_COUNTS  (via has_open_files — check for busy mount)
    //   3. GUEST_MOUNTS  (remove tracking entry)
    //   4. VFS_STATE  (remove mount)
    //   5. GUEST_MOUNTS  (rollback on error, if needed)
    // No two locks are held simultaneously, so deadlock is not possible.

    // Check if it's a guest-created mount.
    let is_guest: bool = GUEST_MOUNTS.lock().iter().any(|m| m.path == mount_path);

    if !is_guest {
        let state = VFS_STATE.lock();
        let vfs: &Vfs = state.as_ref().ok_or(Fat32Error::NotInitialized)?;
        let mount_exists: bool = vfs.mounts().any(|m| m.path() == mount_path);
        if mount_exists {
            return Err(Fat32Error::PermissionDenied);
        } else {
            return Err(Fat32Error::NotFound);
        }
    }

    // Check for open files before modifying any state.
    // FIXME: There is a TOCTOU race between this check and the
    // mount removal below. A concurrent `open()` call could create a
    // new file handle after this check passes. A single-lock design
    // or a mount-level lock would eliminate this window.
    if has_open_files(mount_path) {
        return Err(Fat32Error::FileLocked);
    }

    // Remove from tracking first.
    let info: GuestMountInfo = {
        let mut mounts = GUEST_MOUNTS.lock();
        let pos: usize = mounts
            .iter()
            .position(|m| m.path == mount_path)
            .ok_or(Fat32Error::NotFound)?;
        mounts.remove(pos)
    };

    // Remove from VFS.
    let remove_result = {
        let mut state = VFS_STATE.lock();
        let vfs: &mut Vfs = state.as_mut().ok_or(Fat32Error::NotInitialized)?;
        vfs.remove_mount(mount_path)
    };

    if let Err(e) = remove_result {
        // Rollback: put the tracking info back.
        GUEST_MOUNTS.lock().push(GuestMountInfo {
            path: info.path.clone(),
            memory_ptr: info.memory_ptr,
            memory_size: info.memory_size,
        });
        return Err(e);
    }

    // Free the memory.
    // SAFETY: info.memory_ptr was created from Box::into_raw in
    // create_mount().
    unsafe {
        let _ =
            Box::from_raw(core::ptr::slice_from_raw_parts_mut(info.memory_ptr, info.memory_size));
    }

    Ok(())
}

//==================================================================================================
// Internal Functions
//==================================================================================================

/// Executes a closure with a shared reference to the VFS.
///
/// Locks the global VFS state, verifies it is initialized, and passes
/// a shared reference to the closure. The lock is released when the
/// closure returns.
///
/// # Errors
///
/// Returns [`Fat32Error::NotInitialized`] if `init()` has not been called.
pub(crate) fn with_vfs<F, R>(f: F) -> Result<R, Fat32Error>
where
    F: FnOnce(&Vfs) -> Result<R, Fat32Error>,
{
    let state = VFS_STATE.lock();
    let vfs: &Vfs = state.as_ref().ok_or(Fat32Error::NotInitialized)?;
    f(vfs)
}

/// Executes a closure with a mutable reference to the VFS.
///
/// Locks the global VFS state, verifies it is initialized, and passes
/// a mutable reference to the closure. The lock is released when the
/// closure returns.
///
/// # Errors
///
/// Returns [`Fat32Error::NotInitialized`] if `init()` has not been called.
pub(crate) fn with_vfs_mut<F, R>(f: F) -> Result<R, Fat32Error>
where
    F: FnOnce(&mut Vfs) -> Result<R, Fat32Error>,
{
    let mut state = VFS_STATE.lock();
    let vfs: &mut Vfs = state.as_mut().ok_or(Fat32Error::NotInitialized)?;
    f(vfs)
}

/// Increments the open file count for a mount path.
///
/// Called when a file is successfully opened.
pub(crate) fn increment_open_count(mount_path: &str) {
    let mut counts = OPEN_FILE_COUNTS.lock();
    for entry in counts.iter_mut() {
        if entry.mount_path == mount_path {
            entry.count += 1;
            return;
        }
    }
    counts.push(OpenFileCount {
        mount_path: String::from(mount_path),
        count: 1,
    });
}

/// Decrements the open file count for a mount path.
///
/// Called when a file is closed (dropped).
pub(crate) fn decrement_open_count(mount_path: &str) {
    let mut counts = OPEN_FILE_COUNTS.lock();
    for entry in counts.iter_mut() {
        if entry.mount_path == mount_path {
            entry.count = entry.count.saturating_sub(1);
            return;
        }
    }
}

/// Returns true if there are open files on the given mount.
fn has_open_files(mount_path: &str) -> bool {
    let counts = OPEN_FILE_COUNTS.lock();
    counts
        .iter()
        .any(|e| e.mount_path == mount_path && e.count > 0)
}

/// Formats a memory region as a FAT filesystem.
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` points to valid, writable memory of at least `size` bytes.
/// - The memory is not accessed by other code during formatting.
unsafe fn format_fat_in_memory(ptr: *mut u8, size: usize) -> Result<(), Fat32Error> {
    // SAFETY: Caller guarantees ptr/size validity.
    let mut storage: RawMemoryStorage = unsafe { RawMemoryStorage::new(ptr, size)? };

    let options = ::fatfs::FormatVolumeOptions::new();
    ::fatfs::format_volume(&mut storage, options).map_err(|_| Fat32Error::IoError)?;

    Ok(())
}
