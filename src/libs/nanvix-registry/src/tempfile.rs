// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    warn,
};
use ::std::path::{
    Path,
    PathBuf,
};
use ::tokio::fs;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents a temporary file that is automatically deleted when dropped.
///
/// # Cleanup Behavior
///
/// When this structure is dropped, it attempts to remove the temporary file using synchronous I/O
/// operations. If the cleanup fails (e.g., due to permission issues, the file being in use, or
/// filesystem errors), a warning is logged but no error is propagated. This means that in failure
/// cases, the temporary file may persist on the filesystem and require manual cleanup.
///
/// The destructor uses synchronous I/O because `Drop` cannot be async.
///
pub(crate) struct TemporaryFile {
    /// Path to the temporary file.
    path: PathBuf,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl TemporaryFile {
    ///
    /// # Description
    ///
    /// Creates a new temporary file at the specified path.
    ///
    /// # Parameters
    ///
    /// - `path`: The path where the temporary file will be created.
    ///
    /// # Returns
    ///
    /// A new `TemporaryFile` instance.
    ///
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    ///
    /// # Description
    ///
    /// Gets the path to the temporary file.
    ///
    /// # Returns
    ///
    /// A reference to the path of the temporary file.
    ///
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    ///
    /// # Description
    ///
    /// Writes data to the temporary file.
    ///
    /// # Parameters
    ///
    /// - `contents`: The data to write to the file.
    ///
    /// # Returns
    ///
    /// On success, returns an empty tuple. On failure, returns an object that describes the error.
    ///
    pub(crate) async fn write(&self, contents: &[u8]) -> Result<()> {
        if let Err(error) = fs::write(&self.path, contents).await {
            let reason: String = format!("Failed to write to temporary file: {error}");
            error!("{reason}");
            anyhow::bail!(reason)
        }
        Ok(())
    }
}

impl Drop for TemporaryFile {
    ///
    /// # Description
    ///
    /// Automatically removes the temporary file when the instance is dropped.
    ///
    /// NOTE: This uses synchronous I/O operations since Drop cannot be async. If we spawn an
    /// async task to do this, we risk the task not being executed if the runtime is shut down
    /// before the task runs.
    ///
    fn drop(&mut self) {
        if ::std::fs::metadata(&self.path).is_ok() {
            if let Err(error) = ::std::fs::remove_file(&self.path) {
                warn!("Failed to remove temporary file {:?}: {}", self.path, error);
            }
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::std::env;

    ///
    /// # Description
    ///
    /// Tests temporary file creation.
    ///
    #[test]
    fn test_new() {
        let path: PathBuf = env::temp_dir().join("test-tempfile.txt");
        // Creating TemporaryFile doesn't require tokio runtime until drop.
        let tempfile: TemporaryFile = TemporaryFile { path: path.clone() };
        assert_eq!(tempfile.path(), path.as_path());
        // Prevent drop which requires tokio runtime.
        ::std::mem::forget(tempfile);
    }

    ///
    /// # Description
    ///
    /// Tests that path() returns correct path.
    ///
    #[test]
    fn test_path() {
        let expected_path: PathBuf = PathBuf::from("/tmp/test-file.tar.bz2");
        let tempfile: TemporaryFile = TemporaryFile {
            path: expected_path.clone(),
        };
        assert_eq!(tempfile.path(), expected_path.as_path());
        // Prevent drop which requires tokio runtime.
        ::std::mem::forget(tempfile);
    }

    ///
    /// # Description
    ///
    /// Tests writing to temporary file.
    ///
    #[tokio::test]
    async fn test_write() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempfile-write.txt");
        let tempfile: TemporaryFile = TemporaryFile::new(path.clone());

        let data: &[u8] = b"test data";
        let result: Result<()> = tempfile.write(data).await;
        assert!(result.is_ok());

        // Verify file was written.
        let contents: Result<Vec<u8>, std::io::Error> = fs::read(&path).await;
        assert!(contents.is_ok());
        assert_eq!(contents.expect("failed"), data);

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_file(&path).await;
    }

    ///
    /// # Description
    ///
    /// Tests writing empty data.
    ///
    #[tokio::test]
    async fn test_write_empty() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempfile-empty.txt");
        let tempfile: TemporaryFile = TemporaryFile::new(path.clone());

        let data: &[u8] = b"";
        let result: Result<()> = tempfile.write(data).await;
        assert!(result.is_ok());

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_file(&path).await;
    }

    ///
    /// # Description
    ///
    /// Tests writing large data.
    ///
    #[tokio::test]
    async fn test_write_large() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempfile-large.txt");
        let tempfile: TemporaryFile = TemporaryFile::new(path.clone());

        let data: Vec<u8> = vec![0u8; 1024 * 1024]; // 1 MB.
        let result: Result<()> = tempfile.write(&data).await;
        assert!(result.is_ok());

        // Verify file size.
        let metadata: Result<std::fs::Metadata, std::io::Error> = fs::metadata(&path).await;
        assert!(metadata.is_ok());
        assert_eq!(metadata.expect("failed").len(), 1024 * 1024);

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_file(&path).await;
    }

    ///
    /// # Description
    ///
    /// Tests that the destructor runs even after exiting a scoped tokio runtime.
    ///
    #[test]
    fn test_drop_after_scoped_runtime() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempfile-scoped.txt");

        // Create file in a scoped runtime.
        {
            let runtime: ::tokio::runtime::Runtime =
                ::tokio::runtime::Runtime::new().expect("failed");
            runtime.block_on(async {
                let tempfile: TemporaryFile = TemporaryFile::new(path.clone());
                let result: Result<()> = tempfile.write(b"scoped test data").await;
                assert!(result.is_ok());
                // TemporaryFile will be dropped when exiting this block.
            });
        }

        // Check if file was removed by the destructor.
        let still_exists: bool = path.exists();
        if still_exists {
            // Cleanup before failing.
            let _: Result<(), std::io::Error> = ::std::fs::remove_file(&path);
        }
        assert!(!still_exists, "Temporary file was not automatically removed by destructor");
    }
}
