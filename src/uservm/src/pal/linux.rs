// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Platform abstraction layer for Linux
//!
//! This module provides platform-specific functionalities for Linux-based systems.
//!

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::libc::{
    c_char,
    c_int,
};
use ::log::{
    error,
    trace,
    warn,
};
use ::std::{
    ffi::CString,
    mem,
    os::unix::io::RawFd,
    ptr,
    slice,
};

//==================================================================================================
// Structures
//==================================================================================================

/// An anonymous memory mapping.
#[derive(Debug)]
pub struct AnonymousMapping {
    /// Pointer to the memory location where the region is mapped.
    ptr: *mut ::libc::c_void,
    /// Size of the mapping (in bytes).
    size: usize,
}

// SAFETY: The mapping is an isolated region of memory not shared with other threads.
unsafe impl Send for AnonymousMapping {}
// SAFETY: The &self accessors return borrowed slices tied to &self's lifetime.
unsafe impl Sync for AnonymousMapping {}

/// A memory-mapped file.
#[derive(Debug)]
pub struct FileMapping {
    /// Underlying file descriptor.
    fd: ::libc::c_int,
    /// Pointer to the memory location where the file is mapped.
    ptr: *mut ::libc::c_void,
    /// Size of the mapping (in bytes).
    size: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl AnonymousMapping {
    ///
    /// # Description
    ///
    /// Creates a new anonymous memory mapping.
    ///
    /// # Parameters
    ///
    /// * `size` - Size of the mapping (in bytes). Must be greater than zero.
    /// * `noreserve` - If `true`, do not reserve swap space for the mapping (`MAP_NORESERVE`).
    ///
    /// # Returns
    ///
    /// On success, returns an object representing the anonymous mapping. On failure, returns an
    /// error.
    ///
    pub fn new(size: usize, noreserve: bool) -> Result<Self> {
        trace!("AnonymousMapping::new(): size={size}, noreserve={noreserve}");

        if size == 0 {
            let reason: &str = "cannot create zero-sized anonymous mapping";
            error!("AnonymousMapping::new(): {reason}");
            anyhow::bail!(reason);
        }

        let mut flags: c_int = ::libc::MAP_ANONYMOUS | ::libc::MAP_PRIVATE;
        if noreserve {
            flags |= ::libc::MAP_NORESERVE;
        }

        let ptr: *mut ::libc::c_void = unsafe {
            ::libc::mmap(
                ptr::null_mut(),
                size,
                ::libc::PROT_READ | ::libc::PROT_WRITE,
                flags,
                -1,
                0,
            )
        };

        if ptr == ::libc::MAP_FAILED {
            let reason: String = format!(
                "failed to create anonymous mapping (error={})",
                ::std::io::Error::last_os_error()
            );
            error!("AnonymousMapping::new(): {reason}");
            anyhow::bail!(reason);
        }

        Ok(Self { ptr, size })
    }

    ///
    /// # Description
    ///
    /// Returns a mutable pointer to the mapped memory region.
    ///
    /// # Returns
    ///
    /// A mutable pointer to the mapped memory region.
    ///
    pub fn ptr(&self) -> *mut u8 {
        self.ptr.cast::<u8>()
    }

    ///
    /// # Description
    ///
    /// Returns the size of the mapping (in bytes).
    ///
    /// # Returns
    ///
    /// The size of the mapping (in bytes).
    ///
    pub fn size(&self) -> usize {
        self.size
    }

    ///
    /// # Description
    ///
    /// Returns the mapping contents as an immutable byte slice.
    ///
    /// # Returns
    ///
    /// An immutable byte slice covering the entire mapping.
    ///
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: The mapping is valid for `self.size` bytes for the lifetime of `self`.
        unsafe { slice::from_raw_parts(self.ptr.cast::<u8>(), self.size) }
    }

    ///
    /// # Description
    ///
    /// Returns the mapping contents as a mutable byte slice.
    ///
    /// # Returns
    ///
    /// A mutable byte slice covering the entire mapping.
    ///
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: The mapping is valid for `self.size` bytes for the lifetime of `self`.
        unsafe { slice::from_raw_parts_mut(self.ptr.cast::<u8>(), self.size) }
    }

    ///
    /// # Description
    ///
    /// Replaces the entire mapping with a file-backed read-write mapping using `MAP_FIXED`.
    /// Only the backing store changes; the pointer and size remain the same.
    ///
    /// # Parameters
    ///
    /// * `fd` - File descriptor of the backing file.
    /// * `file_offset` - Byte offset into the file where the mapping begins (must be
    ///   page-aligned).
    ///
    /// # Returns
    ///
    /// On success, returns empty. On failure, returns an error.
    ///
    pub fn remap_file(&self, fd: RawFd, file_offset: ::libc::off_t) -> Result<()> {
        self.remap_file_at(0, self.size, fd, file_offset)
    }

    ///
    /// # Description
    ///
    /// Replaces a sub-region of the mapping with a file-backed read-write mapping using
    /// `MAP_FIXED`. The region `[start, start + len)` must lie within the mapping bounds.
    ///
    /// # Parameters
    ///
    /// * `start` - Byte offset from the start of the mapping (must be page-aligned).
    /// * `len` - Size of the region to remap (in bytes).
    /// * `fd` - File descriptor of the backing file.
    /// * `file_offset` - Byte offset into the file where the mapping begins (must be
    ///   page-aligned).
    ///
    /// # Returns
    ///
    /// On success, returns empty. On failure, returns an error.
    ///
    pub fn remap_file_at(
        &self,
        start: usize,
        len: usize,
        fd: RawFd,
        file_offset: ::libc::off_t,
    ) -> Result<()> {
        trace!(
            "AnonymousMapping::remap_file_at(): start={start:#x}, len={len:#x}, fd={fd}, \
             file_offset={file_offset}"
        );

        if len == 0 {
            let reason: &str = "cannot remap zero-sized region";
            error!("AnonymousMapping::remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        if start.checked_add(len).is_none_or(|end| end > self.size) {
            let reason: String = format!(
                "remap region [{start:#x}, {:#x}) exceeds mapping bounds (size={:#x})",
                start.saturating_add(len),
                self.size
            );
            error!("AnonymousMapping::remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        // SAFETY: `start` has been bounds-checked, so `self.ptr + start` stays within the mapping.
        let addr: *mut u8 = unsafe { self.ptr.cast::<u8>().add(start) };
        let result: *mut u8 = unsafe {
            ::libc::mmap(
                addr.cast::<::libc::c_void>(),
                len,
                ::libc::PROT_READ | ::libc::PROT_WRITE,
                ::libc::MAP_PRIVATE | ::libc::MAP_FIXED,
                fd,
                file_offset,
            )
            .cast::<u8>()
        };

        if result == ::libc::MAP_FAILED.cast::<u8>() {
            let reason: String = format!(
                "failed to remap region as file-backed at {:?} (error={})",
                addr,
                ::std::io::Error::last_os_error()
            );
            error!("AnonymousMapping::remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        debug_assert_eq!(result, addr, "MAP_FIXED should return the exact requested address");

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Issues an `madvise` hint for a sub-region of the mapping.
    ///
    /// # Parameters
    ///
    /// * `start` - Byte offset from the start of the mapping (must be page-aligned).
    /// * `len` - Size of the region in bytes.
    /// * `advice` - madvise advice constant (e.g., `MADV_SEQUENTIAL`, `MADV_WILLNEED`).
    ///
    /// # Returns
    ///
    /// On success, returns empty. On failure, returns an error.
    ///
    pub fn madvise_at(&self, start: usize, len: usize, advice: i32) -> Result<()> {
        trace!("AnonymousMapping::madvise_at(): start={start:#x}, len={len:#x}, advice={advice}");

        if len == 0 {
            return Ok(());
        }

        if !start.is_multiple_of(page_size()) {
            let reason: String = format!(
                "start offset {start:#x} is not page-aligned (page_size={:#x})",
                page_size()
            );
            error!("AnonymousMapping::madvise_at(): {reason}");
            anyhow::bail!(reason);
        }

        if start.checked_add(len).is_none_or(|end| end > self.size) {
            let reason: String = format!(
                "madvise region [{start:#x}, {:#x}) exceeds mapping bounds (size={:#x})",
                start.saturating_add(len),
                self.size
            );
            error!("AnonymousMapping::madvise_at(): {reason}");
            anyhow::bail!(reason);
        }

        // SAFETY: `start` has been bounds-checked, so `self.ptr + start` stays within the mapping.
        let addr: *mut ::libc::c_void = unsafe { self.ptr.cast::<u8>().add(start).cast() };
        let ret: i32 = unsafe { ::libc::madvise(addr, len, advice) };
        if ret != 0 {
            let reason: String = format!(
                "madvise failed at {addr:?} (start={start:#x}, len={len:#x}, advice={advice}, \
                 error={})",
                ::std::io::Error::last_os_error()
            );
            error!("AnonymousMapping::madvise_at(): {reason}");
            anyhow::bail!(reason);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Replaces the entire mapping with a fresh anonymous read-write mapping using `MAP_FIXED`.
    /// This is useful for restoring a neutral memory region after a failed file-backed remap.
    ///
    /// # Returns
    ///
    /// On success, returns empty. On failure, returns an error.
    ///
    pub fn remap_anonymous(&self) -> Result<()> {
        trace!("AnonymousMapping::remap_anonymous(): size={:#x}", self.size);

        let result: *mut u8 = unsafe {
            ::libc::mmap(
                self.ptr,
                self.size,
                ::libc::PROT_READ | ::libc::PROT_WRITE,
                ::libc::MAP_PRIVATE | ::libc::MAP_ANONYMOUS | ::libc::MAP_FIXED,
                -1,
                0,
            )
            .cast::<u8>()
        };

        if result == ::libc::MAP_FAILED.cast::<u8>() {
            let reason: String = format!(
                "failed to restore anonymous mapping at {:?} (error={})",
                self.ptr,
                ::std::io::Error::last_os_error()
            );
            error!("AnonymousMapping::remap_anonymous(): {reason}");
            anyhow::bail!(reason);
        }

        debug_assert_eq!(
            result,
            self.ptr.cast::<u8>(),
            "MAP_FIXED should return the exact requested address"
        );

        Ok(())
    }
}

impl Drop for AnonymousMapping {
    fn drop(&mut self) {
        trace!("drop(): {self:?}");
        unsafe {
            if ::libc::munmap(self.ptr, self.size) < 0 {
                let errno: c_int = *::libc::__errno_location();
                warn!("drop(): failed to unmap anonymous region (errno={errno}, self={self:?})");
            }
        }
    }
}

impl FileMapping {
    ///
    /// # Description
    ///
    /// Maps a file into memory.
    ///
    /// # Parameters
    ///
    /// * `filename` - Name of the file to be mapped.
    ///
    /// # Return Value
    ///
    /// On success, this function returns an object representing the memory-mapped file. On failure,
    /// an error object that describes the error is returned instead.
    ///
    pub fn mmap(filename: &str) -> Result<Self> {
        trace!("mmap(): filename={filename}");

        // Open the file.
        let fd: i32 = unsafe {
            let filename: CString = CString::new(filename)?;
            let filename: &[u8] = filename.as_bytes_with_nul();
            ::libc::open(filename.as_ptr().cast::<c_char>(), ::libc::O_RDONLY)
        };

        // Check if we failed to open the file.
        if fd < 0 {
            let errno: c_int = unsafe { *::libc::__errno_location() };
            let reason: String = format!("failed to open file (errno={errno})");
            error!("mmap(): {reason} (filename={filename})");
            anyhow::bail!(reason);
        }

        // Get file size.
        let size: usize = unsafe {
            let mut stat: ::libc::stat = mem::zeroed();
            let last_errno: c_int = *::libc::__errno_location();
            if ::libc::fstat(fd, &mut stat) < 0 {
                if ::libc::close(fd) < 0 {
                    let errno: c_int = *::libc::__errno_location();
                    warn!("mmap(): failed to close file (errno={errno}, filename={filename})");
                    // Don't bail to report the original error.
                }
                let reason: String = format!("failed to get file size (errno={last_errno})");
                error!("mmap(): {reason} (filename={filename})");
                anyhow::bail!(reason);
            }

            // Convert file size to usize.
            match usize::try_from(stat.st_size) {
                Ok(size) => size,
                Err(_error) => {
                    if ::libc::close(fd) < 0 {
                        let errno: c_int = *::libc::__errno_location();
                        warn!("mmap(): failed to close file (errno={errno}, filename={filename})");
                        // Don't bail to report the original error.
                    }
                    let reason: String = format!("file is too large (size={})", stat.st_size);
                    error!("mmap(): {reason} (filename={filename})");
                    anyhow::bail!(reason);
                },
            }
        };

        // Map the file.
        let ptr: *mut std::ffi::c_void = unsafe {
            ::libc::mmap(ptr::null_mut(), size, ::libc::PROT_READ, ::libc::MAP_PRIVATE, fd, 0)
        };

        // Check if we failed to map the file.
        if std::ptr::eq(ptr, ::libc::MAP_FAILED) {
            let last_errno: c_int = unsafe { *::libc::__errno_location() };
            unsafe {
                if ::libc::close(fd) < 0 {
                    let errno: c_int = *::libc::__errno_location();
                    warn!("mmap(): failed to close file (errno={errno}, filename={filename})");
                    // Don't bail to report the original error.
                }
            }
            let reason: String = format!("failed to map file (errno={last_errno})");
            error!("mmap(): {reason} (filename={filename})");
            anyhow::bail!(reason);
        }

        Ok(Self { fd, size, ptr })
    }

    ///
    /// # Description
    ///
    /// Returns a pointer to the memory-mapped file.
    ///
    /// # Return Value
    ///
    /// A pointer to the memory-mapped file.
    ///
    pub fn ptr(&self) -> *const u8 {
        self.ptr as *const u8
    }

    ///
    /// # Description
    ///
    /// Returns the size of the memory-mapped file (in bytes).
    ///
    /// # Return Value
    ///
    /// The size of the memory-mapped file (in bytes).
    ///
    pub fn size(&self) -> usize {
        self.size
    }

    ///
    /// # Description
    ///
    /// Returns the mapped file contents as an immutable byte slice.
    ///
    /// # Returns
    ///
    /// An immutable byte slice covering the entire mapped file.
    ///
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: The mapping is valid for `self.size` bytes for the lifetime of `self`.
        unsafe { slice::from_raw_parts(self.ptr.cast::<u8>(), self.size) }
    }
}

impl Drop for FileMapping {
    fn drop(&mut self) {
        trace!("drop(): {self:?}");
        unsafe {
            // Attempt to unmap file and check for errors.
            if ::libc::munmap(self.ptr, self.size) < 0 {
                let errno: c_int = *::libc::__errno_location();
                warn!("drop(): failed to unmap file (errno={errno}, self={self:?})");
                // Don't bail to attempt to close the file.
            }
            // Attempt to close file and check for errors.
            if ::libc::close(self.fd) < 0 {
                let errno: c_int = *::libc::__errno_location();
                warn!("drop(): failed to close file (errno={errno}, self={self:?})");
                // Don't bail to report the original error.
            }
        }
    }
}

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Returns the system page size (cached after the first call).
///
/// # Returns
///
/// The system page size (in bytes).
///
fn page_size() -> usize {
    static PAGE_SIZE: std::sync::LazyLock<usize> = std::sync::LazyLock::new(|| {
        // SAFETY: `sysconf(_SC_PAGESIZE)` is always safe to call and returns a positive value.
        #[allow(clippy::cast_possible_truncation)]
        let size: usize = unsafe { ::libc::sysconf(::libc::_SC_PAGESIZE) as usize };
        size
    });
    *PAGE_SIZE
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::anyhow::Result;
    use ::std::{
        env,
        fs,
        path::PathBuf,
        process,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    fn unique_temp_path(prefix: &str) -> Result<PathBuf> {
        let timestamp: u128 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let pid: u32 = process::id();
        let mut path: PathBuf = env::temp_dir();
        let filename: ::std::string::String = format!("{prefix}-{pid}-{timestamp}");
        path.push(filename);
        Ok(path)
    }

    #[test]
    fn test_mmap_reads_file_contents() -> Result<()> {
        let path: PathBuf = unique_temp_path("nanvix-filemapping-test")?;
        let content: &[u8] = b"nanvix mmap test content";
        fs::write(&path, content)?;

        let path_string: ::std::string::String = path.to_string_lossy().into_owned();
        let mapping: FileMapping = FileMapping::mmap(path_string.as_str())?;

        let expected_size: usize = content.len();
        assert_eq!(mapping.size(), expected_size, "mapping size mismatch");

        let ptr: *const u8 = mapping.ptr();
        assert!(!ptr.is_null(), "mapping pointer is null");

        let mapped: &[u8] = unsafe { ::std::slice::from_raw_parts(ptr, mapping.size()) };
        assert_eq!(mapped, content, "mapped bytes differ from file contents");

        drop(mapping);
        fs::remove_file(&path)?;

        Ok(())
    }

    #[test]
    fn test_mmap_rejects_zero_sized_files() -> Result<()> {
        let path: PathBuf = unique_temp_path("nanvix-filemapping-empty")?;
        let _file: fs::File = fs::File::create(&path)?;

        let path_string: ::std::string::String = path.to_string_lossy().into_owned();
        let result: Result<FileMapping> = FileMapping::mmap(path_string.as_str());
        assert!(result.is_err(), "mmap should fail for zero-length files");

        fs::remove_file(&path)?;

        Ok(())
    }

    #[test]
    fn test_file_mapping_as_slice() -> Result<()> {
        let path: PathBuf = unique_temp_path("nanvix-filemapping-as-slice")?;
        let content: &[u8] = b"as_slice test content for FileMapping";
        fs::write(&path, content)?;

        let path_string: ::std::string::String = path.to_string_lossy().into_owned();
        let mapping: FileMapping = FileMapping::mmap(path_string.as_str())?;

        let slice: &[u8] = mapping.as_slice();
        assert_eq!(slice.len(), content.len(), "slice length mismatch");
        assert_eq!(slice, content, "slice contents differ from file contents");

        drop(mapping);
        fs::remove_file(&path)?;

        Ok(())
    }

    #[test]
    fn test_anonymous_mapping_allocates_and_drops() -> Result<()> {
        let size: usize = 4096;
        let mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

        assert!(!mapping.ptr().is_null(), "mapping pointer is null");
        assert_eq!(mapping.size(), size, "mapping size mismatch");

        // Verify the region is writable and readable.
        let data: &mut [u8] = unsafe { slice::from_raw_parts_mut(mapping.ptr(), size) };
        data[0] = 0xAB;
        data[size - 1] = 0xCD;
        assert_eq!(data[0], 0xAB, "first byte mismatch");
        assert_eq!(data[size - 1], 0xCD, "last byte mismatch");

        drop(mapping);

        Ok(())
    }

    #[test]
    fn test_anonymous_mapping_noreserve() -> Result<()> {
        let size: usize = 4096;
        let mapping: AnonymousMapping = AnonymousMapping::new(size, true)?;

        assert!(!mapping.ptr().is_null(), "mapping pointer is null");
        assert_eq!(mapping.size(), size, "mapping size mismatch");

        drop(mapping);

        Ok(())
    }

    #[test]
    fn test_anonymous_mapping_zero_size_fails() {
        let result: Result<AnonymousMapping> = AnonymousMapping::new(0, false);
        assert!(result.is_err(), "zero-sized anonymous mapping should fail");
    }

    #[test]
    fn test_anonymous_mapping_as_slice() -> Result<()> {
        let size: usize = 4096;
        let mut mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

        // Write via mutable slice.
        let mutable_slice: &mut [u8] = mapping.as_mut_slice();
        mutable_slice[0] = 0x42;
        mutable_slice[size - 1] = 0x99;

        // Read via immutable slice.
        let immutable_slice: &[u8] = mapping.as_slice();
        assert_eq!(immutable_slice[0], 0x42, "first byte mismatch via as_slice");
        assert_eq!(immutable_slice[size - 1], 0x99, "last byte mismatch via as_slice");
        assert_eq!(immutable_slice.len(), size, "slice length mismatch");

        Ok(())
    }

    #[test]
    fn test_remap_file_and_remap_anonymous() -> Result<()> {
        let path: PathBuf = unique_temp_path("nanvix-remap-file")?;
        let content: &[u8] = b"remap file test data!!!!";
        fs::write(&path, content)?;

        let size: usize = 4096;
        let mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

        // Remap the mapping to be file-backed.
        let file: fs::File = fs::File::open(&path)?;
        let fd: RawFd = ::std::os::unix::io::AsRawFd::as_raw_fd(&file);
        mapping.remap_file(fd, 0)?;

        // Verify the file contents are visible.
        let mapped: &[u8] = &mapping.as_slice()[..content.len()];
        assert_eq!(mapped, content, "file-backed remap should expose file contents");

        // Restore to anonymous.
        mapping.remap_anonymous()?;

        // After restoring, memory should be zeroed.
        let zeroed: &[u8] = mapping.as_slice();
        assert!(zeroed.iter().all(|&b| b == 0), "anonymous remap should zero memory");

        drop(mapping);
        drop(file);
        fs::remove_file(&path)?;

        Ok(())
    }

    #[test]
    fn test_remap_file_at_sub_region() -> Result<()> {
        let path: PathBuf = unique_temp_path("nanvix-remap-at")?;
        let content: Vec<u8> = vec![0xAB; 4096];
        fs::write(&path, &content)?;

        let size: usize = 2 * 4096;
        let mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

        // Remap only the second page.
        let file: fs::File = fs::File::open(&path)?;
        let fd: RawFd = ::std::os::unix::io::AsRawFd::as_raw_fd(&file);
        mapping.remap_file_at(4096, 4096, fd, 0)?;

        // First page should still be zeroed (anonymous).
        let first_page: &[u8] = &mapping.as_slice()[..4096];
        assert!(first_page.iter().all(|&b| b == 0), "first page should remain anonymous");

        // Second page should have file contents.
        let second_page: &[u8] = &mapping.as_slice()[4096..8192];
        assert_eq!(second_page, &content[..], "second page should have file contents");

        drop(mapping);
        drop(file);
        fs::remove_file(&path)?;

        Ok(())
    }

    #[test]
    fn test_remap_file_at_rejects_out_of_bounds() {
        let mapping: AnonymousMapping =
            AnonymousMapping::new(4096, false).expect("failed to create mapping");
        let result: Result<()> = mapping.remap_file_at(0, 8192, -1, 0);
        assert!(result.is_err(), "remap_file_at should reject out-of-bounds region");
    }

    #[test]
    fn test_remap_file_at_rejects_zero_len() {
        let mapping: AnonymousMapping =
            AnonymousMapping::new(4096, false).expect("failed to create mapping");
        let result: Result<()> = mapping.remap_file_at(0, 0, -1, 0);
        assert!(result.is_err(), "remap_file_at should reject zero-length region");
    }

    #[test]
    fn test_madvise_at_success() -> Result<()> {
        let size: usize = 2 * 4096;
        let mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

        // Valid madvise on a page-aligned sub-region should succeed.
        mapping.madvise_at(0, 4096, ::libc::MADV_WILLNEED)?;
        mapping.madvise_at(4096, 4096, ::libc::MADV_SEQUENTIAL)?;

        Ok(())
    }

    #[test]
    fn test_madvise_at_zero_len_is_noop() -> Result<()> {
        let mapping: AnonymousMapping = AnonymousMapping::new(4096, false)?;

        // Zero-length madvise should return Ok without doing anything.
        mapping.madvise_at(0, 0, ::libc::MADV_WILLNEED)?;

        Ok(())
    }

    #[test]
    fn test_madvise_at_rejects_out_of_bounds() {
        let mapping: AnonymousMapping =
            AnonymousMapping::new(4096, false).expect("failed to create mapping");
        let result: Result<()> = mapping.madvise_at(0, 8192, ::libc::MADV_WILLNEED);
        assert!(result.is_err(), "madvise_at should reject out-of-bounds region");
    }

    #[test]
    fn test_madvise_at_rejects_unaligned_start() {
        let mapping: AnonymousMapping =
            AnonymousMapping::new(8192, false).expect("failed to create mapping");
        let result: Result<()> = mapping.madvise_at(1, 4096, ::libc::MADV_WILLNEED);
        assert!(result.is_err(), "madvise_at should reject non-page-aligned start");
    }
}
