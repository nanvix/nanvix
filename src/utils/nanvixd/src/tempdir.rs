// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::nanvix::log::{
    error,
    trace,
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
/// Represents a temporary directory that is automatically deleted when dropped.
///
/// # Cleanup Behavior
///
/// When this structure is dropped, it attempts to remove the temporary directory and all its
/// contents using synchronous I/O operations. If the cleanup fails (e.g., due to permission
/// issues, the directory being in use, or filesystem errors), a warning is logged but no error is
/// propagated. This means that in failure cases, the temporary directory may persist on the
/// filesystem and require manual cleanup.
///
/// The destructor uses synchronous I/O because `Drop` cannot be async. For large directory trees,
/// this may briefly block the current thread during cleanup.
///
pub struct TemporaryDirectory {
    /// Path to the temporary directory.
    path: PathBuf,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl TemporaryDirectory {
    ///
    /// # Description
    ///
    /// Creates a new temporary directory at the specified path.
    ///
    /// # Parameters
    ///
    /// - `path`: The path where the temporary directory will be created.
    ///
    /// # Returns
    ///
    /// On success, returns a new `TemporaryDirectory` instance. On failure, returns an error
    /// describing what went wrong during directory creation.
    ///
    pub async fn new(path: PathBuf) -> Result<Self> {
        if let Err(error) = fs::create_dir_all(&path).await {
            let reason: String =
                format!("Failed to create temporary directory '{}': {}", path.display(), error);
            error!("new(): {reason}");
            anyhow::bail!(reason)
        }
        Ok(Self { path })
    }

    ///
    /// # Description
    ///
    /// Gets the path to the temporary directory.
    ///
    /// # Returns
    ///
    /// A reference to the path of the temporary directory.
    ///
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryDirectory {
    ///
    /// # Description
    ///
    /// Automatically removes the temporary directory when the instance is dropped.
    ///
    /// NOTE: This uses synchronous I/O operations since Drop cannot be async. For large directory
    /// trees, this may briefly block the current thread. If we spawn an async task to do this, we
    /// risk the task not being executed if the runtime is shut down before the task runs.
    fn drop(&mut self) {
        trace!("drop(): self.path={:?}", self.path);
        if ::std::fs::metadata(&self.path).is_ok() {
            if let Err(error) = ::std::fs::remove_dir_all(&self.path) {
                warn!("drop(): failed to remove temporary directory {:?}: {}", self.path, error);
            }
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::std::env;

    ///
    /// # Description
    ///
    /// Tests that `new()` creates a `TemporaryDirectory` instance with the correct path.
    ///
    #[tokio::test]
    async fn test_new() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempdir-new");
        let tempdir: TemporaryDirectory = TemporaryDirectory::new(path.clone()).await.unwrap();
        assert_eq!(tempdir.path(), path.as_path());

        // Verify directory was created.
        assert!(
            fs::metadata(&path).await.is_ok(),
            "Temporary directory does not exist after creation"
        );
        assert!(fs::metadata(&path).await.unwrap().is_dir(), "Created path is not a directory");

        // Manual cleanup: We use mem::forget to prevent automatic Drop cleanup, then manually
        // remove the directory. This ensures tests don't interfere with each other and gives us
        // explicit control over cleanup timing.
        fs::remove_dir_all(&path).await.ok();
        ::std::mem::forget(tempdir);
    }

    ///
    /// # Description
    ///
    /// Tests that `path()` returns the correct path to the temporary directory.
    ///
    #[tokio::test]
    async fn test_path() {
        let expected_path: PathBuf = env::temp_dir().join("nanvix-test-tempdir-path");
        let tempdir: TemporaryDirectory = TemporaryDirectory::new(expected_path.clone())
            .await
            .unwrap();
        assert_eq!(tempdir.path(), expected_path.as_path());

        // Manual cleanup: We use mem::forget to prevent automatic Drop cleanup, then manually
        // remove the directory. This ensures tests don't interfere with each other and gives us
        // explicit control over cleanup timing.
        fs::remove_dir_all(&expected_path).await.ok();
        ::std::mem::forget(tempdir);
    }

    ///
    /// # Description
    ///
    /// Tests that `new()` successfully creates a temporary directory on the filesystem.
    ///
    #[tokio::test]
    async fn test_create() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempdir-create");
        let tempdir: TemporaryDirectory = TemporaryDirectory::new(path.clone()).await.unwrap();

        // Verify directory was created.
        assert!(
            fs::metadata(&path).await.is_ok(),
            "Temporary directory does not exist after creation"
        );
        assert!(fs::metadata(&path).await.unwrap().is_dir(), "Created path is not a directory");

        // Manual cleanup: We use mem::forget to prevent automatic Drop cleanup, then manually
        // remove the directory. This ensures tests don't interfere with each other and gives us
        // explicit control over cleanup timing.
        fs::remove_dir_all(&path).await.ok();
        ::std::mem::forget(tempdir);
    }

    ///
    /// # Description
    ///
    /// Tests that `new()` successfully creates nested temporary directories.
    ///
    #[tokio::test]
    async fn test_create_nested() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempdir-nested/sub1/sub2");
        let tempdir: TemporaryDirectory = TemporaryDirectory::new(path.clone()).await.unwrap();

        // Verify directory was created.
        assert!(
            fs::metadata(&path).await.is_ok(),
            "Nested temporary directory does not exist after creation"
        );
        assert!(
            fs::metadata(&path).await.unwrap().is_dir(),
            "Created nested path is not a directory"
        );

        // Manual cleanup: We use mem::forget to prevent automatic Drop cleanup, then manually
        // remove the parent directory. This ensures tests don't interfere with each other and
        // gives us explicit control over cleanup timing.
        let parent: PathBuf = env::temp_dir().join("nanvix-test-tempdir-nested");
        fs::remove_dir_all(&parent).await.ok();
        ::std::mem::forget(tempdir);
    }

    ///
    /// # Description
    ///
    /// Tests that `new()` succeeds when the directory already exists.
    ///
    #[tokio::test]
    async fn test_create_existing() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempdir-existing");

        // Pre-create directory.
        fs::create_dir_all(&path).await.ok();

        let tempdir: TemporaryDirectory = TemporaryDirectory::new(path.clone()).await.unwrap();

        // Manual cleanup: We use mem::forget to prevent automatic Drop cleanup, then manually
        // remove the directory. This ensures tests don't interfere with each other and gives us
        // explicit control over cleanup timing.
        fs::remove_dir_all(&path).await.ok();
        ::std::mem::forget(tempdir);
    }

    ///
    /// # Description
    ///
    /// Tests that `new()` fails gracefully when directory creation is not possible.
    ///
    #[tokio::test]
    async fn test_create_failure() {
        // Use an invalid path that cannot be created.
        let path: PathBuf = PathBuf::from("/proc/invalid/path/that/cannot/be/created");
        let result: Result<TemporaryDirectory> = TemporaryDirectory::new(path).await;

        assert!(result.is_err(), "Expected new() to fail for invalid path");
    }

    ///
    /// # Description
    ///
    /// Tests that the temporary directory is cleaned up when dropped.
    ///
    #[tokio::test]
    async fn test_drop_cleanup() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempdir-drop");

        {
            let _tempdir: TemporaryDirectory = TemporaryDirectory::new(path.clone()).await.unwrap();

            // Verify directory exists before drop.
            assert!(fs::metadata(&path).await.is_ok(), "Directory should exist before drop");
        }

        // Give some time for the async cleanup to complete.
        ::tokio::time::sleep(::std::time::Duration::from_millis(100)).await;

        // Verify directory was removed after drop.
        assert!(fs::metadata(&path).await.is_err(), "Directory should not exist after drop");
    }

    ///
    /// # Description
    ///
    /// Tests that drop handles non-existent directories gracefully.
    ///
    #[tokio::test]
    async fn test_drop_non_existent() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempdir-nonexistent");

        {
            let _tempdir: TemporaryDirectory = TemporaryDirectory::new(path.clone()).await.unwrap();
            // Manually remove the directory before drop.
            fs::remove_dir_all(&path).await.ok();
        }

        // Give some time for the async cleanup to complete.
        ::tokio::time::sleep(::std::time::Duration::from_millis(100)).await;

        // Should not panic or error, just no-op.
        assert!(fs::metadata(&path).await.is_err(), "Directory should not exist");
    }

    ///
    /// # Description
    ///
    /// Tests that the destructor runs even after exiting a scoped tokio runtime.
    ///
    #[test]
    fn test_drop_after_runtime_exit() {
        let path: PathBuf = env::temp_dir().join("nanvix-test-tempdir-runtime-exit");

        // Create a scoped tokio runtime.
        {
            let runtime: ::tokio::runtime::Runtime = ::tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let _tempdir: TemporaryDirectory =
                    TemporaryDirectory::new(path.clone()).await.unwrap();

                // Verify directory exists.
                assert!(
                    fs::metadata(&path).await.is_ok(),
                    "Directory should exist within runtime scope"
                );

                // Drop occurs here when exiting async block.
            });

            // Runtime is still alive here but async context has exited.
        }

        // Runtime has been dropped, give time for any spawned cleanup tasks.
        ::std::thread::sleep(::std::time::Duration::from_millis(200));

        // Verify directory was removed even after runtime exit.
        let metadata_result: ::std::io::Result<::std::fs::Metadata> = ::std::fs::metadata(&path);
        if metadata_result.is_ok() {
            // Clean up manually before failing.
            ::std::fs::remove_dir_all(&path).ok();
            panic!("Directory still exists after runtime exit, destructor did not run properly");
        }
    }
}
