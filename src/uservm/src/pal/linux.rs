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
use ::std::{
    ffi::CString,
    mem,
    ptr,
};
use ::syslog::{
    error,
    trace,
    warn,
};

//==================================================================================================
// Structures
//==================================================================================================

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
}
