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
use ::std::{
    fs,
    path::Path,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A file loaded into memory.
#[derive(Debug)]
pub struct FileMapping {
    /// Contents of the file.
    data: Vec<u8>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl FileMapping {
    ///
    /// # Description
    ///
    /// Reads a file into memory.
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

        let data: Vec<u8> = match fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                let reason: String = format!("failed to read file (error={e})");
                error!("open(): {reason} (filename={filename})");
                anyhow::bail!(reason);
            },
        };

        Ok(Self { data })
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
        self.data.as_ptr()
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
        self.data.len()
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
