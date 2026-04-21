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
    warn,
};
use ::std::{
    fs::{
        self,
        File,
    },
    os::windows::io::AsRawHandle,
    path::Path,
};
use ::windows::Win32::{
    Foundation::{
        CloseHandle,
        HANDLE,
    },
    System::Memory::{
        CreateFileMappingW,
        FILE_MAP_READ,
        MEMORY_MAPPED_VIEW_ADDRESS,
        MapViewOfFile,
        PAGE_READONLY,
        UnmapViewOfFile,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

/// A memory-mapped file.
pub struct FileMapping {
    /// Section handle returned by `CreateFileMappingW`.
    section_handle: HANDLE,
    /// File view mapped into host address space by `MapViewOfFile`.
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    /// Size of the mapped region (in bytes).
    size: usize,
}

// SAFETY: `FileMapping` owns OS handles (section handle, mapped view) that have no thread
// affinity. All mutation requires `&mut self`, and resources are released exactly once in `Drop`.
unsafe impl Send for FileMapping {}
unsafe impl Sync for FileMapping {}

//==================================================================================================
// Implementations
//==================================================================================================

impl FileMapping {
    ///
    /// # Description
    ///
    /// Maps a file into memory (read-only).
    ///
    /// # Parameters
    ///
    /// * `filename` - Name of the file to be loaded.
    ///
    /// # Returns
    ///
    /// On success, this function returns an object representing the memory-mapped file. On failure,
    /// an error object that describes the error is returned instead.
    ///
    pub fn open(filename: &str) -> Result<Self> {
        trace!("open(): filename={filename}");

        let path: &Path = Path::new(filename);

        let file: File = fs::File::open(path).map_err(|e| {
            let reason: String = format!("failed to open file (error={e})");
            error!("open(): {reason} (filename={filename})");
            anyhow::anyhow!(reason)
        })?;

        let size: usize = usize::try_from(
            file.metadata()
                .map_err(|e| {
                    let reason: String = format!("failed to get file metadata (error={e})");
                    error!("open(): {reason} (filename={filename})");
                    anyhow::anyhow!(reason)
                })?
                .len(),
        )
        .map_err(|_| {
            let reason: &str = "file size exceeds addressable range";
            error!("open(): {reason} (filename={filename})");
            anyhow::anyhow!(reason)
        })?;

        if size == 0 {
            let reason: &str = "cannot map zero-sized file";
            error!("open(): {reason} (filename={filename})");
            anyhow::bail!(reason);
        }

        let file_handle: HANDLE = HANDLE(file.as_raw_handle());

        // NOTE: `file` is dropped at the end of this function, but the section handle created below
        // keeps an internal kernel reference to the underlying file object.  The OS file handle can
        // therefore be closed safely; the mapping stays valid until `section_handle` itself is
        // closed in `Drop`.
        //
        // SAFETY: `file_handle` is a valid OS handle obtained from the `File` opened above.
        // Passing `None` for security attributes and name is permitted. Size parameters of
        // (0, 0) tell the OS to use the file's actual size.
        let section_handle: HANDLE = unsafe {
            CreateFileMappingW(file_handle, None, PAGE_READONLY, 0, 0, None).map_err(|e| {
                let reason: String = format!("failed to create file mapping (error={e:?})");
                error!("open(): {reason} (filename={filename})");
                anyhow::anyhow!(reason)
            })?
        };

        // SAFETY: `section_handle` is a valid section handle from the successful
        // `CreateFileMappingW` call above. `size` equals the file size obtained from metadata.
        let view: MEMORY_MAPPED_VIEW_ADDRESS =
            unsafe { MapViewOfFile(section_handle, FILE_MAP_READ, 0, 0, size) };

        if view.Value.is_null() {
            // SAFETY: `section_handle` is a valid handle from `CreateFileMappingW`; it must be
            // closed before returning, since the view creation failed.
            unsafe {
                if CloseHandle(section_handle).is_err() {
                    warn!("open(): CloseHandle() failed while cleaning up section handle");
                }
            }
            let reason: &str = "MapViewOfFile returned null";
            error!("open(): {reason} (filename={filename})");
            anyhow::bail!(reason);
        }

        Ok(Self {
            section_handle,
            view,
            size,
        })
    }

    ///
    /// # Description
    ///
    /// Returns a pointer to the mapped file data.
    ///
    /// # Returns
    ///
    /// A pointer to the file data.
    ///
    pub fn ptr(&self) -> *const u8 {
        self.view.Value as *const u8
    }

    ///
    /// # Description
    ///
    /// Returns the size of the mapped file (in bytes).
    ///
    /// # Returns
    ///
    /// The size of the file (in bytes).
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
        unsafe { ::std::slice::from_raw_parts(self.view.Value as *const u8, self.size) }
    }
}

impl ::std::fmt::Debug for FileMapping {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.debug_struct("FileMapping")
            .field("view", &self.view.Value)
            .field("size", &self.size)
            .finish()
    }
}

impl Drop for FileMapping {
    fn drop(&mut self) {
        trace!("drop(): FileMapping (size={})", self.size);
        // SAFETY: `self.view` and `self.section_handle` are valid OS resources created in
        // `open()`. They are released exactly once here and not used after this point.
        unsafe {
            if let Err(e) = UnmapViewOfFile(self.view) {
                error!("drop(): UnmapViewOfFile failed (error={e:?})");
            }
            if let Err(e) = CloseHandle(self.section_handle) {
                error!("drop(): CloseHandle failed on section handle (error={e:?})");
            }
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

        // Drop the mapping before deleting the file; the section handle keeps the file open.
        drop(mapping);
        fs::remove_file(path_buf).ok();
        Ok(())
    }

    #[test]
    fn open_as_slice_returns_file_contents() -> Result<()> {
        let (path_str, path_buf): (String, PathBuf) = unique_temp_path("as-slice")?;
        let payload: &[u8] = b"as_slice test content for FileMapping";
        fs::write(&path_buf, payload)?;

        let mapping: FileMapping = FileMapping::open(&path_str)?;

        let slice: &[u8] = mapping.as_slice();
        assert_eq!(slice.len(), payload.len(), "slice length mismatch");
        assert_eq!(slice, payload, "slice contents differ from file contents");

        drop(mapping);
        fs::remove_file(path_buf).ok();
        Ok(())
    }

    #[test]
    fn open_returns_error_for_missing_file() {
        let result: Result<FileMapping> = FileMapping::open("/non/existent/path/to/file");
        assert!(result.is_err());
    }
}
