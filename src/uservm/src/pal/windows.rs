// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Platform abstraction layer for Windows
//!
//! This module provides platform-specific functionalities for Windows-based systems.
//!

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
use ::std::os::windows::ffi::OsStrExt;
use ::std::path::Path;
use windows::Win32::{
    Foundation::{
        CloseHandle,
        HANDLE,
        INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::{
        CreateFileW,
        GetFileSizeEx,
        FILE_ATTRIBUTE_NORMAL,
        FILE_GENERIC_READ,
        FILE_SHARE_READ,
        OPEN_EXISTING,
    },
    System::Memory::{
        CreateFileMappingW,
        MapViewOfFile,
        UnmapViewOfFile,
        FILE_MAP_READ,
        PAGE_READONLY,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

/// A file loaded into memory via Windows memory-mapped I/O.
#[derive(Debug)]
pub struct FileMapping {
    /// Pointer to the mapped view.
    ptr: *const u8,
    /// Size of the file in bytes.
    len: usize,
    /// Handle to the file mapping object (needed for cleanup).
    map_handle: HANDLE,
    /// Handle to the file (needed for cleanup).
    file_handle: HANDLE,
}

// SAFETY: The mapped memory is read-only and the handles are opaque OS resources.
unsafe impl Send for FileMapping {}
unsafe impl Sync for FileMapping {}

//==================================================================================================
// Implementations
//==================================================================================================

impl FileMapping {
    ///
    /// # Description
    ///
    /// Memory-maps a file for reading using Windows `CreateFileMapping`/`MapViewOfFile`.
    ///
    /// # Parameters
    ///
    /// * `filename` - Name of the file to be loaded.
    ///
    /// # Returns
    ///
    /// On success, this function returns an object representing the loaded file. On failure,
    /// an error object that describes the error is returned instead.
    ///
    pub fn open(filename: &str) -> Result<Self> {
        trace!("open(): filename={filename}");

        let path: &Path = Path::new(filename);
        let wide_path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            // Open the file for reading.
            let file_handle: HANDLE = CreateFileW(
                windows::core::PCWSTR(wide_path.as_ptr()),
                FILE_GENERIC_READ.0,
                FILE_SHARE_READ,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
            .map_err(|e| {
                let reason: String = format!("failed to open file (error={e})");
                error!("open(): {reason} (filename={filename})");
                anyhow::anyhow!(reason)
            })?;

            if file_handle == INVALID_HANDLE_VALUE {
                anyhow::bail!("open(): CreateFileW returned INVALID_HANDLE_VALUE");
            }

            // Get file size.
            let mut file_size_large: i64 = 0;
            GetFileSizeEx(file_handle, &mut file_size_large).map_err(|e| {
                let _ = CloseHandle(file_handle);
                let reason: String = format!("failed to get file size (error={e})");
                error!("open(): {reason} (filename={filename})");
                anyhow::anyhow!(reason)
            })?;

            let file_size: usize = file_size_large as usize;

            if file_size == 0 {
                let _ = CloseHandle(file_handle);
                anyhow::bail!("open(): file is empty (filename={filename})");
            }

            // Create a file mapping object.
            let map_handle: HANDLE =
                CreateFileMappingW(file_handle, None, PAGE_READONLY, 0, 0, None).map_err(|e| {
                    let _ = CloseHandle(file_handle);
                    let reason: String = format!("failed to create file mapping (error={e})");
                    error!("open(): {reason} (filename={filename})");
                    anyhow::anyhow!(reason)
                })?;

            // Map the file into memory.
            let view = MapViewOfFile(map_handle, FILE_MAP_READ, 0, 0, 0);
            if view.Value.is_null() {
                let _ = CloseHandle(map_handle);
                let _ = CloseHandle(file_handle);
                anyhow::bail!("open(): MapViewOfFile returned null (filename={filename})");
            }

            trace!("open(): mapped {file_size} bytes from {filename}");

            Ok(Self {
                ptr: view.Value as *const u8,
                len: file_size,
                map_handle,
                file_handle,
            })
        }
    }

    ///
    /// # Description
    ///
    /// Returns a pointer to the loaded file data.
    ///
    /// # Returns
    ///
    /// A pointer to the file data.
    ///
    pub fn ptr(&self) -> *const u8 {
        self.ptr
    }

    ///
    /// # Description
    ///
    /// Returns the size of the loaded file (in bytes).
    ///
    /// # Returns
    ///
    /// The size of the file (in bytes).
    ///
    pub fn size(&self) -> usize {
        self.len
    }
}

impl Drop for FileMapping {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(windows::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.ptr as *mut _,
            });
            let _ = CloseHandle(self.map_handle);
            let _ = CloseHandle(self.file_handle);
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
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

    /// Returns a unique file path in the system temp directory for test isolation.
    fn unique_temp_path(suffix: &str) -> Result<(String, PathBuf)> {
        let mut path: PathBuf = env::temp_dir();
        let nanos: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| anyhow::anyhow!("failed to compute timestamp (error={:?})", error))?
            .as_nanos();
        let file_name: String =
            format!("nanvix-pal-test-{}-{}-{}.tmp", process::id(), nanos, suffix);
        path.push(&file_name);
        Ok((path.to_string_lossy().into_owned(), path))
    }

    #[test]
    fn open_returns_file_contents() -> Result<()> {
        let (path_str, path_buf): (String, PathBuf) = unique_temp_path("open")?;
        let payload: &[u8] = b"hello world";
        fs::write(&path_buf, payload)?;

        let mapping: FileMapping = FileMapping::open(&path_str)?;
        assert_eq!(mapping.size(), payload.len());

        let loaded: &[u8] = unsafe { ::std::slice::from_raw_parts(mapping.ptr(), mapping.size()) };
        assert_eq!(loaded, payload);

        fs::remove_file(path_buf).ok();
        Ok(())
    }

    #[test]
    fn open_returns_error_for_missing_file() {
        let result: Result<FileMapping> = FileMapping::open("/non/existent/path/to/file");
        assert!(result.is_err());
    }
}
