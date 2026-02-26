// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Global filesystem state management.
//!
//! This module manages the global VFS state and provides mount/unmount
//! operations for FAT filesystems. The guest application must call [`init()`]
//! before using any filesystem operations, then use [`mount()`] or
//! [`create_mount()`] to add FAT filesystems.
//!
//! # Thread Safety
//!
//! All global state uses `UnsafeCell` with manual `Sync` implementations.
//! This is safe because nanvix guest applications are single-threaded.

//==================================================================================================
// Imports
//==================================================================================================

use alloc::{
    boxed::Box,
    string::String,
    vec::Vec,
};
use core::cell::UnsafeCell;

use crate::{
    error::FsError,
    fat::{
        Fat,
        RawMemoryStorage,
    },
    vfs::{
        Mount,
        Vfs,
    },
};

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

/// Global VFS state.
///
/// # Safety
///
/// Guest code is single-threaded, so this is safe.
static VFS_STATE: VfsStateCell = VfsStateCell(UnsafeCell::new(None));

struct VfsStateCell(UnsafeCell<Option<Vfs>>);

// SAFETY: Guest is single-threaded.
unsafe impl Sync for VfsStateCell {}

impl VfsStateCell {
    fn get(&self) -> Option<&Vfs> {
        // SAFETY: Guest is single-threaded.
        unsafe { (*self.0.get()).as_ref() }
    }

    fn is_initialized(&self) -> bool {
        // SAFETY: Guest is single-threaded.
        unsafe { (*self.0.get()).is_some() }
    }

    fn set(&self, state: Vfs) {
        // SAFETY: Guest is single-threaded.
        unsafe {
            *self.0.get() = Some(state);
        }
    }
}

/// Tracks guest-created mounts for unmount permission checks and
/// memory deallocation.
static GUEST_MOUNTS: GuestMountsCell = GuestMountsCell(UnsafeCell::new(Vec::new()));

struct GuestMountsCell(UnsafeCell<Vec<GuestMountInfo>>);

// SAFETY: Guest is single-threaded.
unsafe impl Sync for GuestMountsCell {}

/// Information about a guest-created mount.
struct GuestMountInfo {
    /// Mount path (e.g., "/scratch").
    path: String,
    /// Pointer to the allocated memory (for deallocation).
    memory_ptr: *mut u8,
    /// Size of the allocated memory.
    memory_size: usize,
}

impl GuestMountsCell {
    fn add(&self, path: String, ptr: *mut u8, size: usize) {
        // SAFETY: Guest is single-threaded.
        unsafe {
            (*self.0.get()).push(GuestMountInfo {
                path,
                memory_ptr: ptr,
                memory_size: size,
            });
        }
    }

    fn contains(&self, path: &str) -> bool {
        // SAFETY: Guest is single-threaded.
        unsafe { (*self.0.get()).iter().any(|m| m.path == path) }
    }

    fn remove(&self, path: &str) -> Option<GuestMountInfo> {
        // SAFETY: Guest is single-threaded.
        unsafe {
            let mounts: &mut Vec<GuestMountInfo> = &mut *self.0.get();
            mounts
                .iter()
                .position(|m| m.path == path)
                .map(|pos| mounts.remove(pos))
        }
    }
}

/// Tracks the number of open files per mount path.
static OPEN_FILE_COUNTS: OpenFileCountsCell = OpenFileCountsCell(UnsafeCell::new(Vec::new()));

struct OpenFileCountsCell(UnsafeCell<Vec<OpenFileCount>>);

// SAFETY: Guest is single-threaded.
unsafe impl Sync for OpenFileCountsCell {}

struct OpenFileCount {
    mount_path: String,
    count: usize,
}

impl OpenFileCountsCell {
    fn increment(&self, mount_path: &str) {
        // SAFETY: Guest is single-threaded.
        unsafe {
            let counts: &mut Vec<OpenFileCount> = &mut *self.0.get();
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
    }

    fn decrement(&self, mount_path: &str) {
        // SAFETY: Guest is single-threaded.
        unsafe {
            let counts: &mut Vec<OpenFileCount> = &mut *self.0.get();
            for entry in counts.iter_mut() {
                if entry.mount_path == mount_path {
                    entry.count = entry.count.saturating_sub(1);
                    return;
                }
            }
        }
    }

    fn has_open_files(&self, mount_path: &str) -> bool {
        // SAFETY: Guest is single-threaded.
        unsafe {
            let counts: &Vec<OpenFileCount> = &*self.0.get();
            counts
                .iter()
                .any(|e| e.mount_path == mount_path && e.count > 0)
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
/// Returns [`FsError::NotSupported`] if the filesystem is already
/// initialized.
pub fn init() -> Result<(), FsError> {
    if VFS_STATE.is_initialized() {
        return Err(FsError::NotSupported);
    }

    VFS_STATE.set(Vfs::new());
    Ok(())
}

/// Returns true if the filesystem is initialized.
pub fn is_initialized() -> bool {
    VFS_STATE.is_initialized()
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
/// - [`FsError::NotInitialized`] if `init()` has not been called.
/// - [`FsError::InvalidPath`] if `mount_path` doesn't start with "/".
/// - [`FsError::InvalidArgument`] if `ptr` is null or `size` is zero.
/// - [`FsError::AlreadyExists`] if a mount already exists at this path.
/// - [`FsError::IoError`] if the FAT image is invalid or corrupted.
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` points to valid memory containing a FAT filesystem image.
/// - The memory remains valid for the lifetime of the mount.
/// - The memory region is at least `size` bytes.
pub unsafe fn mount(mount_path: &str, ptr: *mut u8, size: usize) -> Result<(), FsError> {
    if !mount_path.starts_with('/') {
        return Err(FsError::InvalidPath);
    }

    // SAFETY: Caller guarantees memory region validity.
    let fat: Fat = unsafe { Fat::from_memory(ptr, size)? };
    let mount: Mount = Mount::new(String::from(mount_path), fat)?;

    // SAFETY: Single-threaded guest.
    let vfs: &mut Vfs = unsafe { vfs_mut()? };
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
/// - [`FsError::NotInitialized`] if `init()` has not been called.
/// - [`FsError::InvalidPath`] if `mount_path` doesn't start with "/".
/// - [`FsError::InvalidArgument`] if `size` is out of range.
/// - [`FsError::AlreadyExists`] if a mount already exists at this path.
/// - [`FsError::IoError`] if formatting fails.
pub fn create_mount(mount_path: &str, size: usize) -> Result<(), FsError> {
    if !mount_path.starts_with('/') {
        return Err(FsError::InvalidPath);
    }

    if size < MIN_FAT_SIZE {
        return Err(FsError::InvalidArgument);
    }
    if size > MAX_FAT_SIZE {
        return Err(FsError::InvalidArgument);
    }

    // Check VFS is initialized.
    let _ = vfs()?;

    // Allocate memory for the FAT image.
    let memory: Box<[u8]> = alloc::vec![0u8; size].into_boxed_slice();
    let memory_ptr: *mut u8 = Box::into_raw(memory) as *mut u8;

    // Format the memory as FAT.
    // SAFETY: memory_ptr points to valid, zeroed memory of `size` bytes.
    let format_result: Result<(), FsError> = unsafe { format_fat_in_memory(memory_ptr, size) };

    if let Err(e) = format_result {
        // SAFETY: memory_ptr was created from Box::into_raw above.
        unsafe {
            let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(memory_ptr, size));
        }
        return Err(e);
    }

    // Create Fat from the formatted memory.
    // SAFETY: memory_ptr points to valid FAT image of `size` bytes.
    let fat: Fat = match unsafe { Fat::from_memory(memory_ptr, size) } {
        Ok(fat) => fat,
        Err(e) => {
            // SAFETY: memory_ptr was created from Box::into_raw above.
            unsafe {
                let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(memory_ptr, size));
            }
            return Err(e);
        },
    };

    let mount: Mount = match Mount::new(String::from(mount_path), fat) {
        Ok(mount) => mount,
        Err(e) => {
            // SAFETY: memory_ptr was created from Box::into_raw above.
            unsafe {
                let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(memory_ptr, size));
            }
            return Err(e);
        },
    };

    // Add to VFS.
    // SAFETY: Not holding any other VFS references.
    let vfs: &mut Vfs = unsafe { vfs_mut()? };
    if let Err(e) = vfs.add_mount(mount) {
        // SAFETY: memory_ptr was created from Box::into_raw above.
        unsafe {
            let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(memory_ptr, size));
        }
        return Err(e);
    }

    // Track this as a guest-created mount.
    GUEST_MOUNTS.add(String::from(mount_path), memory_ptr, size);

    Ok(())
}

/// Unmounts a guest-created FAT mount and frees its memory.
///
/// Only mounts created via [`create_mount()`] can be unmounted.
/// Attempting to unmount a mount created via [`mount()`] will fail with
/// [`FsError::PermissionDenied`].
///
/// # Parameters
///
/// - `mount_path`: The path of the mount to remove.
///
/// # Errors
///
/// - [`FsError::NotInitialized`] if `init()` has not been called.
/// - [`FsError::NotFound`] if no mount exists at this path.
/// - [`FsError::PermissionDenied`] if mount was not created by
///   [`create_mount()`].
/// - [`FsError::FileLocked`] if files are still open on this mount.
pub fn unmount(mount_path: &str) -> Result<(), FsError> {
    if !GUEST_MOUNTS.contains(mount_path) {
        let vfs: &Vfs = vfs()?;
        let mount_exists: bool = vfs.mounts().any(|m| m.path() == mount_path);
        if mount_exists {
            return Err(FsError::PermissionDenied);
        } else {
            return Err(FsError::NotFound);
        }
    }

    // Check for open files before modifying any state.
    if OPEN_FILE_COUNTS.has_open_files(mount_path) {
        return Err(FsError::FileLocked);
    }

    // Remove from tracking first.
    let info: GuestMountInfo = GUEST_MOUNTS.remove(mount_path).ok_or(FsError::NotFound)?;

    // Remove from VFS.
    // SAFETY: Not holding any other VFS references.
    let vfs: &mut Vfs = unsafe { vfs_mut()? };
    if let Err(e) = vfs.remove_mount(mount_path) {
        // Rollback: put the tracking info back.
        GUEST_MOUNTS.add(info.path.clone(), info.memory_ptr, info.memory_size);
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

/// Gets a reference to the VFS.
///
/// # Errors
///
/// Returns [`FsError::NotInitialized`] if `init()` has not been called.
pub(crate) fn vfs() -> Result<&'static Vfs, FsError> {
    VFS_STATE.get().ok_or(FsError::NotInitialized)
}

/// Gets a mutable reference to the VFS.
///
/// # Safety
///
/// The caller must ensure that no other references (mutable or immutable)
/// to the VFS exist when calling this function.
///
/// # Errors
///
/// Returns [`FsError::NotInitialized`] if `init()` has not been called.
pub(crate) unsafe fn vfs_mut() -> Result<&'static mut Vfs, FsError> {
    // SAFETY: Caller guarantees no aliasing references exist.
    let state: &mut Option<Vfs> = unsafe { &mut *VFS_STATE.0.get() };
    state.as_mut().ok_or(FsError::NotInitialized)
}

/// Increments the open file count for a mount path.
///
/// Called when a file is successfully opened.
pub(crate) fn increment_open_count(mount_path: &str) {
    OPEN_FILE_COUNTS.increment(mount_path);
}

/// Decrements the open file count for a mount path.
///
/// Called when a file is closed (dropped).
pub(crate) fn decrement_open_count(mount_path: &str) {
    OPEN_FILE_COUNTS.decrement(mount_path);
}

/// Formats a memory region as a FAT filesystem.
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` points to valid, writable memory of at least `size` bytes.
/// - The memory is not accessed by other code during formatting.
unsafe fn format_fat_in_memory(ptr: *mut u8, size: usize) -> Result<(), FsError> {
    // SAFETY: Caller guarantees ptr/size validity.
    let mut storage: RawMemoryStorage = unsafe { RawMemoryStorage::new(ptr, size)? };

    let options = ::fatfs::FormatVolumeOptions::new();
    ::fatfs::format_volume(&mut storage, options).map_err(|_| FsError::IoError)?;

    Ok(())
}
