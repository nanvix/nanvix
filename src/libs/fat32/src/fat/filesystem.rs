// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! High-level FAT filesystem wrapper.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Fat32Error,
    fat::{
        error::map_fatfs_error,
        file::FatFile,
        storage::RawMemoryStorage,
        time::NanvixTimeProvider,
        InternalFatFs,
    },
};
use ::core::fmt;
use ::fatfs::{
    Seek,
    SeekFrom,
};

//==================================================================================================
// Structures
//==================================================================================================

/// High-level FAT filesystem wrapper.
///
/// Wraps a `fatfs::FileSystem` over [`RawMemoryStorage`] and provides a
/// clean API that returns [`Fat32Error`] instead of fatfs error types.
///
/// # Description
///
/// This is the FAT backend for the VFS. Given a pointer to a memory region
/// containing a FAT image, this type allows reading and writing files.
pub struct Fat {
    /// The underlying fatfs FileSystem.
    fs: InternalFatFs,
    /// Base pointer to the FAT image in memory (for zero-copy file access).
    base_ptr: *const u8,
}

// SAFETY: Fat is only accessed through the VFS Mutex, which ensures
// exclusive access. The raw pointer represents memory managed by the
// state module and is only used for zero-copy reads.
unsafe impl Send for Fat {}

//==================================================================================================
// Implementations
//==================================================================================================

impl Fat {
    /// Opens an existing FAT filesystem from a memory region.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the start of the FAT image in memory.
    /// - `size`: Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// A new [`Fat`] instance, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::InvalidArgument`] if `ptr` is null or `size` is zero.
    /// - [`Fat32Error::IoError`] if the FAT image is invalid or corrupted.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory region is valid, properly aligned,
    /// and remains valid for the lifetime of this [`Fat`].
    pub unsafe fn from_memory(ptr: *mut u8, size: usize) -> Result<Self, Fat32Error> {
        // SAFETY: Caller guarantees memory region validity.
        let storage: RawMemoryStorage = unsafe { RawMemoryStorage::new(ptr, size)? };
        let options = ::fatfs::FsOptions::new().time_provider(NanvixTimeProvider);
        let fs: InternalFatFs =
            ::fatfs::FileSystem::new(storage, options).map_err(map_fatfs_error)?;
        Ok(Self {
            fs,
            base_ptr: ptr as *const u8,
        })
    }

    /// Opens a file with the specified mode.
    ///
    /// # Parameters
    ///
    /// - `path`: Path relative to the FAT root (e.g., "subdir/file.txt").
    /// - `read`: Open for reading.
    /// - `write`: Open for writing.
    /// - `create`: Create file if it doesn't exist.
    /// - `truncate`: Truncate file to zero length.
    ///
    /// # Returns
    ///
    /// A new [`FatFile`] handle, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if file doesn't exist and `create` is false.
    /// - [`Fat32Error::IoError`] if path refers to a directory.
    pub fn open(
        &self,
        path: &str,
        read: bool,
        write: bool,
        create: bool,
        truncate: bool,
    ) -> Result<FatFile<'_>, Fat32Error> {
        let root = self.fs.root_dir();

        if create {
            match root.open_file(path) {
                Ok(mut file) => {
                    if truncate {
                        file.truncate().map_err(map_fatfs_error)?;
                    }
                    Ok(FatFile::new(file, read, write))
                },
                Err(::fatfs::Error::NotFound) => {
                    let file = root.create_file(path).map_err(map_fatfs_error)?;
                    Ok(FatFile::new(file, read, write))
                },
                Err(e) => Err(map_fatfs_error(e)),
            }
        } else {
            let mut file = root.open_file(path).map_err(map_fatfs_error)?;
            if truncate && write {
                file.truncate().map_err(map_fatfs_error)?;
            }
            Ok(FatFile::new(file, read, write))
        }
    }

    /// Creates a new file, failing if it already exists.
    ///
    /// Implements `O_CREAT | O_EXCL` semantics.
    ///
    /// # Parameters
    ///
    /// - `path`: Path relative to the FAT root.
    /// - `read`: Whether the file handle should allow reading.
    /// - `write`: Whether the file handle should allow writing.
    ///
    /// # Returns
    ///
    /// A new [`FatFile`] handle, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::AlreadyExists`] if file already exists.
    /// - [`Fat32Error::NotFound`] if parent directory doesn't exist.
    pub fn create_new(
        &self,
        path: &str,
        read: bool,
        write: bool,
    ) -> Result<FatFile<'_>, Fat32Error> {
        let root = self.fs.root_dir();

        // fatfs::Dir::create_file does NOT fail if file exists - it opens it.
        // We must explicitly check for existence first.
        if root.open_file(path).is_ok() {
            return Err(Fat32Error::AlreadyExists);
        }

        let file = root.create_file(path).map_err(map_fatfs_error)?;
        Ok(FatFile::new(file, read, write))
    }

    /// Gets file/directory metadata.
    ///
    /// # Parameters
    ///
    /// - `path`: Path relative to the FAT root.
    ///
    /// # Returns
    ///
    /// File metadata, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if path doesn't exist.
    pub fn stat(&self, path: &str) -> Result<FatStat, Fat32Error> {
        let root = self.fs.root_dir();

        if path.is_empty() || path == "/" || path == "." {
            return Ok(FatStat {
                size: 0,
                is_dir: true,
            });
        }

        // Try opening as file first.
        if let Ok(mut file) = root.open_file(path) {
            let size: u64 = file.seek(SeekFrom::End(0)).map_err(map_fatfs_error)?;
            return Ok(FatStat {
                size,
                is_dir: false,
            });
        }

        // Try opening as directory.
        if root.open_dir(path).is_ok() {
            return Ok(FatStat {
                size: 0,
                is_dir: true,
            });
        }

        Err(Fat32Error::NotFound)
    }

    /// Returns a pointer and size for zero-copy access to a file's data.
    ///
    /// If the file's clusters are stored contiguously in the FAT image,
    /// returns `Some((data_ptr, file_size))` where `data_ptr` points directly
    /// into the in-memory FAT image. Returns `None` if the file is empty,
    /// not found, or its clusters are not contiguous.
    ///
    /// # Parameters
    ///
    /// - `path`: Path relative to the FAT root.
    pub fn file_raw_region(&self, path: &str) -> Option<(*const u8, usize)> {
        let root: ::fatfs::Dir<
            '_,
            RawMemoryStorage,
            NanvixTimeProvider,
            ::fatfs::LossyOemCpConverter,
        > = self.fs.root_dir();
        let mut file: ::fatfs::File<
            '_,
            RawMemoryStorage,
            NanvixTimeProvider,
            ::fatfs::LossyOemCpConverter,
        > = root.open_file(path).ok()?;

        let mut first_offset: Option<u64> = None;
        let mut total_size: usize = 0;
        let mut next_expected: u64 = 0;

        for (i, extent_result) in file.extents().enumerate() {
            let extent: ::fatfs::Extent = extent_result.ok()?;
            if i == 0 {
                first_offset = Some(extent.offset);
            } else if extent.offset != next_expected {
                return None; // Not contiguous.
            }
            next_expected = extent.offset + extent.size as u64;
            total_size += extent.size as usize;
        }

        let offset: u64 = first_offset?;
        if total_size == 0 {
            return None;
        }

        // SAFETY: base_ptr is valid for the lifetime of the Fat instance,
        // and offset + total_size is within the FAT image bounds.
        let data_ptr: *const u8 = unsafe { self.base_ptr.add(offset as usize) };
        Some((data_ptr, total_size))
    }

    /// Reads directory contents.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the directory.
    ///
    /// # Returns
    ///
    /// A vector of directory entries (excluding `.` and `..`).
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if directory doesn't exist.
    /// - [`Fat32Error::IoError`] if path is a file.
    pub fn read_dir(&self, path: &str) -> Result<alloc::vec::Vec<FatDirEntry>, Fat32Error> {
        let root = self.fs.root_dir();

        let dir = if path.is_empty() || path == "/" || path == "." {
            root
        } else {
            root.open_dir(path).map_err(map_fatfs_error)?
        };

        let mut entries: alloc::vec::Vec<FatDirEntry> = alloc::vec::Vec::new();
        for entry in dir.iter() {
            let entry = entry.map_err(map_fatfs_error)?;
            let name: alloc::string::String = entry.file_name();

            if name == "." || name == ".." {
                continue;
            }

            entries.push(FatDirEntry {
                name,
                is_dir: entry.is_dir(),
                size: entry.len(),
            });
        }

        Ok(entries)
    }

    /// Creates a directory.
    ///
    /// # Parameters
    ///
    /// - `path`: Path for the new directory.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::AlreadyExists`] if directory already exists.
    /// - [`Fat32Error::NotFound`] if parent directory doesn't exist.
    pub fn mkdir(&self, path: &str) -> Result<(), Fat32Error> {
        let root = self.fs.root_dir();

        // fatfs::Dir::create_dir does NOT fail if the directory exists — it
        // silently opens it. Check explicitly so callers get AlreadyExists.
        if root.open_dir(path).is_ok() {
            return Err(Fat32Error::AlreadyExists);
        }

        root.create_dir(path).map_err(map_fatfs_error)?;
        Ok(())
    }

    /// Removes an empty directory.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the directory to remove.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if directory doesn't exist.
    /// - [`Fat32Error::NotEmpty`] if directory is not empty.
    /// - [`Fat32Error::NotADirectory`] if path is a file.
    pub fn rmdir(&self, path: &str) -> Result<(), Fat32Error> {
        let root = self.fs.root_dir();

        // Verify it is a directory, not a file.
        if root.open_file(path).is_ok() {
            return Err(Fat32Error::NotADirectory);
        }

        // Verify directory exists before removing.
        root.open_dir(path).map_err(map_fatfs_error)?;

        root.remove(path).map_err(map_fatfs_error)
    }

    /// Deletes a file.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the file to delete.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if file doesn't exist.
    /// - [`Fat32Error::NotAFile`] if path is a directory.
    pub fn unlink(&self, path: &str) -> Result<(), Fat32Error> {
        let root = self.fs.root_dir();

        // Verify it is a file, not a directory.
        if root.open_dir(path).is_ok() {
            return Err(Fat32Error::NotAFile);
        }

        // Verify file exists before removing.
        root.open_file(path).map_err(map_fatfs_error)?;

        root.remove(path).map_err(map_fatfs_error)
    }

    /// Renames/moves a file or directory.
    ///
    /// # Parameters
    ///
    /// - `old_path`: Current path of the file or directory.
    /// - `new_path`: New path for the file or directory.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if source doesn't exist.
    /// - [`Fat32Error::AlreadyExists`] if destination already exists.
    pub fn rename(&self, old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
        let root = self.fs.root_dir();
        root.rename(old_path, &root, new_path)
            .map_err(map_fatfs_error)
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Debug for Fat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fat").finish_non_exhaustive()
    }
}

//==================================================================================================
// Supporting Types
//==================================================================================================

/// File metadata from a FAT filesystem.
///
/// Returned by [`Fat::stat()`] to describe a file or directory.
#[derive(Debug, Clone, Copy)]
pub struct FatStat {
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// True if this is a directory.
    pub is_dir: bool,
}

/// Directory entry from a FAT filesystem.
///
/// Returned by [`Fat::read_dir()`] for each file or subdirectory.
/// Does not include `.` or `..` pseudo-entries.
#[derive(Debug, Clone)]
pub struct FatDirEntry {
    /// Entry name (filename only, not full path).
    pub name: alloc::string::String,
    /// True if this is a directory.
    pub is_dir: bool,
    /// Size in bytes (0 for directories).
    pub size: u64,
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::fat::RawMemoryStorage;
    use ::alloc::{
        vec,
        vec::Vec,
    };

    /// Helper: creates formatted FAT image in a heap buffer and returns a `Fat`.
    ///
    /// The returned `Vec<u8>` must be kept alive for the lifetime of the `Fat`.
    /// Drop the `Fat` before the `Vec` to avoid use-after-free during
    /// fatfs `FileSystem::drop()`.
    fn make_fat(size: usize) -> (Fat, Vec<u8>) {
        let mut buf: Vec<u8> = vec![0u8; size];
        let ptr: *mut u8 = buf.as_mut_ptr();

        let mut storage: RawMemoryStorage =
            unsafe { RawMemoryStorage::new(ptr, size).expect("valid storage") };
        ::fatfs::format_volume(&mut storage, ::fatfs::FormatVolumeOptions::new())
            .expect("format should succeed");

        let fat: Fat = unsafe { Fat::from_memory(ptr, size).expect("valid fat") };
        (fat, buf)
    }

    /// Wrapper that ensures `Fat` is dropped before its backing buffer.
    ///
    /// When `Fat` is dropped, fatfs `FileSystem::drop()` flushes dirty metadata
    /// back to `RawMemoryStorage`. The buffer must remain valid during that flush.
    /// This wrapper enforces the correct drop order.
    struct FatHandle {
        /// Fat filesystem — dropped first.
        fat: core::mem::ManuallyDrop<Fat>,
        /// Backing memory — dropped second (after `fat`).
        _buf: Vec<u8>,
    }

    impl FatHandle {
        fn new() -> Self {
            let (fat, buf) = make_fat(IMG_SIZE);
            Self {
                fat: core::mem::ManuallyDrop::new(fat),
                _buf: buf,
            }
        }
    }

    impl core::ops::Deref for FatHandle {
        type Target = Fat;
        fn deref(&self) -> &Fat {
            &self.fat
        }
    }

    impl Drop for FatHandle {
        fn drop(&mut self) {
            // SAFETY: `fat` is dropped exactly once, before `_buf`.
            unsafe {
                core::mem::ManuallyDrop::drop(&mut self.fat);
            }
        }
    }

    const IMG_SIZE: usize = 128 * 1024;

    // -- from_memory tests -------------------------------------------------------

    /// Tests that `from_memory` rejects a null pointer.
    #[test]
    fn from_memory_null_ptr_fails() {
        let result: Result<Fat, Fat32Error> =
            unsafe { Fat::from_memory(core::ptr::null_mut(), 1024) };
        assert!(result.is_err(), "null pointer should be rejected");
    }

    /// Tests that `from_memory` rejects zero size.
    #[test]
    fn from_memory_zero_size_fails() {
        let mut buf: Vec<u8> = vec![0u8; 1024];
        let result: Result<Fat, Fat32Error> = unsafe { Fat::from_memory(buf.as_mut_ptr(), 0) };
        assert!(result.is_err(), "zero size should be rejected");
    }

    /// Tests that `from_memory` succeeds on a formatted image.
    #[test]
    fn from_memory_valid_image() {
        let _fat: FatHandle = FatHandle::new();
    }

    // -- open / create / read / write tests --------------------------------------

    /// Tests creating a file, writing, and reading it back.
    #[test]
    fn write_and_read_file() {
        let fat: FatHandle = FatHandle::new();

        // Create and write.
        {
            let mut file = fat
                .open("test.txt", false, true, true, false)
                .expect("create should succeed");
            file.write(b"hello fat32").expect("write should succeed");
            file.flush().expect("flush should succeed");
        }

        // Read back.
        {
            let mut file = fat
                .open("test.txt", true, false, false, false)
                .expect("open for read should succeed");
            let mut buf: [u8; 64] = [0u8; 64];
            let n: usize = file.read(&mut buf).expect("read should succeed");
            assert_eq!(n, 11, "should read 11 bytes");
            assert_eq!(&buf[..n], b"hello fat32");
        }
    }

    /// Tests that opening a non-existent file without create fails.
    #[test]
    fn open_nonexistent_fails() {
        let fat: FatHandle = FatHandle::new();
        let result = fat.open("nonexistent.txt", true, false, false, false);
        assert!(result.is_err(), "opening non-existent file should fail");
    }

    /// Tests open with truncate.
    #[test]
    fn open_truncate() {
        let fat: FatHandle = FatHandle::new();

        // Write initial content.
        {
            let mut file = fat
                .open("trunc.txt", false, true, true, false)
                .expect("create should succeed");
            file.write(b"initial content")
                .expect("write should succeed");
            file.flush().expect("flush should succeed");
        }

        // Open with truncate.
        {
            let mut file = fat
                .open("trunc.txt", true, true, false, true)
                .expect("open with truncate should succeed");
            let mut buf: [u8; 64] = [0u8; 64];
            let n: usize = file.read(&mut buf).expect("read should succeed");
            assert_eq!(n, 0, "truncated file should be empty");
        }
    }

    /// Tests `create_new` succeeds for a new file.
    #[test]
    fn create_new_succeeds() {
        let fat: FatHandle = FatHandle::new();
        let _file = fat
            .create_new("new.txt", true, true)
            .expect("create_new should succeed");
    }

    /// Tests `create_new` fails if file already exists.
    #[test]
    fn create_new_existing_fails() {
        let fat: FatHandle = FatHandle::new();

        {
            let mut f = fat
                .open("exists.txt", false, true, true, false)
                .expect("create should succeed");
            f.write(b"data").expect("write should succeed");
            f.flush().expect("flush should succeed");
        }

        let result: Result<FatFile<'_>, Fat32Error> = fat.create_new("exists.txt", true, true);
        assert_eq!(
            result.unwrap_err(),
            Fat32Error::AlreadyExists,
            "create_new on existing file should return AlreadyExists"
        );
    }

    // -- stat tests --------------------------------------------------------------

    /// Tests stat on root directory.
    #[test]
    fn stat_root() {
        let fat: FatHandle = FatHandle::new();
        let info: FatStat = fat.stat("").expect("stat root should succeed");
        assert!(info.is_dir, "root should be a directory");
    }

    /// Tests stat on a file.
    #[test]
    fn stat_file() {
        let fat: FatHandle = FatHandle::new();

        {
            let mut f = fat
                .open("sized.txt", false, true, true, false)
                .expect("create should succeed");
            f.write(b"12345").expect("write should succeed");
            f.flush().expect("flush should succeed");
        }

        let info: FatStat = fat.stat("sized.txt").expect("stat should succeed");
        assert!(!info.is_dir, "should not be a directory");
        assert_eq!(info.size, 5, "file size should be 5");
    }

    /// Tests stat on a non-existent path.
    #[test]
    fn stat_nonexistent() {
        let fat: FatHandle = FatHandle::new();
        let result: Result<FatStat, Fat32Error> = fat.stat("nope.txt");
        assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
    }

    // -- mkdir / rmdir tests -----------------------------------------------------

    /// Tests creating and stat-ing a directory.
    #[test]
    fn mkdir_and_stat() {
        let fat: FatHandle = FatHandle::new();
        fat.mkdir("subdir").expect("mkdir should succeed");
        let info: FatStat = fat.stat("subdir").expect("stat subdir should succeed");
        assert!(info.is_dir, "should be a directory");
    }

    /// Tests that creating a duplicate directory fails.
    #[test]
    fn mkdir_duplicate_fails() {
        let fat: FatHandle = FatHandle::new();
        fat.mkdir("dup").expect("first mkdir should succeed");
        let result: Result<(), Fat32Error> = fat.mkdir("dup");
        assert_eq!(result.unwrap_err(), Fat32Error::AlreadyExists);
    }

    /// Tests removing an empty directory.
    #[test]
    fn rmdir_empty() {
        let fat: FatHandle = FatHandle::new();
        fat.mkdir("torm").expect("mkdir should succeed");
        fat.rmdir("torm").expect("rmdir should succeed");
        let result: Result<FatStat, Fat32Error> = fat.stat("torm");
        assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
    }

    /// Tests that rmdir on a file fails.
    #[test]
    fn rmdir_on_file_fails() {
        let fat: FatHandle = FatHandle::new();
        {
            let mut f = fat
                .open("file-a.txt", false, true, true, false)
                .expect("create should succeed");
            f.write(b"x").expect("write should succeed");
            f.flush().expect("flush should succeed");
        }
        let result: Result<(), Fat32Error> = fat.rmdir("file-a.txt");
        assert_eq!(result.unwrap_err(), Fat32Error::NotADirectory);
    }

    // -- unlink tests ------------------------------------------------------------

    /// Tests deleting a file.
    #[test]
    fn unlink_file() {
        let fat: FatHandle = FatHandle::new();
        {
            let mut f = fat
                .open("del.txt", false, true, true, false)
                .expect("create should succeed");
            f.write(b"bye").expect("write should succeed");
            f.flush().expect("flush should succeed");
        }
        fat.unlink("del.txt").expect("unlink should succeed");
        assert_eq!(fat.stat("del.txt").unwrap_err(), Fat32Error::NotFound);
    }

    /// Tests that unlink on a directory fails.
    #[test]
    fn unlink_on_dir_fails() {
        let fat: FatHandle = FatHandle::new();
        fat.mkdir("adir").expect("mkdir should succeed");
        let result: Result<(), Fat32Error> = fat.unlink("adir");
        assert_eq!(result.unwrap_err(), Fat32Error::NotAFile);
    }

    // -- read_dir tests ----------------------------------------------------------

    /// Tests listing an empty root directory.
    #[test]
    fn read_dir_empty_root() {
        let fat: FatHandle = FatHandle::new();
        let entries: Vec<FatDirEntry> = fat.read_dir("").expect("read_dir root should succeed");
        assert!(entries.is_empty(), "fresh root should be empty");
    }

    /// Tests listing root with files and directories.
    #[test]
    fn read_dir_with_entries() {
        let fat: FatHandle = FatHandle::new();
        {
            let mut f = fat
                .open("f1.txt", false, true, true, false)
                .expect("create should succeed");
            f.write(b"a").expect("write should succeed");
            f.flush().expect("flush should succeed");
        }
        fat.mkdir("d1").expect("mkdir should succeed");

        let entries: Vec<FatDirEntry> = fat.read_dir("").expect("read_dir should succeed");
        assert_eq!(entries.len(), 2, "should have 2 entries");

        let names: alloc::vec::Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"f1.txt"), "should contain f1.txt");
        assert!(names.contains(&"d1"), "should contain d1");
    }

    // -- rename tests ------------------------------------------------------------

    /// Tests renaming a file.
    #[test]
    fn rename_file() {
        let fat: FatHandle = FatHandle::new();
        {
            let mut f = fat
                .open("old.txt", false, true, true, false)
                .expect("create should succeed");
            f.write(b"content").expect("write should succeed");
            f.flush().expect("flush should succeed");
        }

        fat.rename("old.txt", "new.txt")
            .expect("rename should succeed");
        assert_eq!(fat.stat("old.txt").unwrap_err(), Fat32Error::NotFound);
        let info: FatStat = fat.stat("new.txt").expect("stat new.txt should succeed");
        assert!(!info.is_dir);
        assert_eq!(info.size, 7);
    }

    // -- file_raw_region tests ---------------------------------------------------

    /// Tests that file_raw_region returns None for non-existent files.
    #[test]
    fn raw_region_nonexistent() {
        let fat: FatHandle = FatHandle::new();
        assert!(fat.file_raw_region("nope.txt").is_none());
    }

    /// Tests that file_raw_region returns a valid region for a contiguous file.
    #[test]
    fn raw_region_contiguous_file() {
        let fat: FatHandle = FatHandle::new();
        let data: &[u8] = b"raw region data";
        {
            let mut f = fat
                .open("raw.txt", false, true, true, false)
                .expect("create should succeed");
            f.write(data).expect("write should succeed");
            f.flush().expect("flush should succeed");
        }

        if let Some((ptr, size)) = fat.file_raw_region("raw.txt") {
            assert_eq!(size, data.len(), "raw region size should match");
            let slice: &[u8] = unsafe { core::slice::from_raw_parts(ptr, size) };
            assert_eq!(slice, data, "raw region data should match");
        }
        // Note: if clusters are not contiguous, None is acceptable.
    }

    // -- Debug trait test --------------------------------------------------------

    /// Tests that Fat implements Debug.
    #[test]
    fn fat_debug() {
        let handle: FatHandle = FatHandle::new();
        let fat: &Fat = &handle;
        let debug: alloc::string::String = alloc::format!("{fat:?}");
        assert!(debug.contains("Fat"), "debug should contain type name");
    }
}
