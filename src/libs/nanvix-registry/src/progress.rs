// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::sync::Arc;

//==================================================================================================
// Traits
//==================================================================================================

///
/// # Description
///
/// Trait for receiving progress updates during package downloads.
///
/// Implement this trait to receive callbacks during download and installation operations.
///
pub trait ProgressCallback: Send + Sync {
    ///
    /// # Description
    ///
    /// Called when a download starts.
    ///
    /// # Parameters
    ///
    /// - `package_name`: Name of the package being downloaded.
    /// - `total_size`: Total size in bytes, if known.
    ///
    fn on_download_start(&self, package_name: &str, total_size: Option<u64>);

    ///
    /// # Description
    ///
    /// Called when a download completes successfully.
    ///
    /// # Parameters
    ///
    /// - `package_name`: Name of the package that was downloaded.
    ///
    fn on_download_complete(&self, package_name: &str);

    ///
    /// # Description
    ///
    /// Called when extraction/installation starts.
    ///
    /// # Parameters
    ///
    /// - `package_name`: Name of the package being extracted.
    ///
    fn on_extract_start(&self, package_name: &str);

    ///
    /// # Description
    ///
    /// Called when extraction/installation completes.
    ///
    /// # Parameters
    ///
    /// - `package_name`: Name of the package that was extracted.
    ///
    fn on_extract_complete(&self, package_name: &str);

    ///
    /// # Description
    ///
    /// Called when installing a dependency.
    ///
    /// # Parameters
    ///
    /// - `dependency_name`: Name of the dependency being installed.
    /// - `parent_package`: Name of the package that requires this dependency.
    ///
    fn on_dependency_start(&self, dependency_name: &str, parent_package: &str);
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A no-op progress callback that does nothing.
///
/// This is the default progress callback used when no callback is provided.
///
#[derive(Debug, Clone, Default)]
pub struct NoOpProgress;

impl ProgressCallback for NoOpProgress {
    fn on_download_start(&self, _package_name: &str, _total_size: Option<u64>) {}

    fn on_download_complete(&self, _package_name: &str) {}

    fn on_extract_start(&self, _package_name: &str) {}

    fn on_extract_complete(&self, _package_name: &str) {}

    fn on_dependency_start(&self, _dependency_name: &str, _parent_package: &str) {}
}

///
/// # Description
///
/// A simple logging progress callback that logs progress via the `log` crate.
///
#[derive(Debug, Clone, Default)]
pub struct LoggingProgress;

impl ProgressCallback for LoggingProgress {
    fn on_download_start(&self, package_name: &str, total_size: Option<u64>) {
        if let Some(size) = total_size {
            ::log::info!("Starting download of {} ({} bytes)", package_name, size);
        } else {
            ::log::info!("Starting download of {}", package_name);
        }
    }

    fn on_download_complete(&self, package_name: &str) {
        ::log::info!("Download complete: {}", package_name);
    }

    fn on_extract_start(&self, package_name: &str) {
        ::log::info!("Extracting: {}", package_name);
    }

    fn on_extract_complete(&self, package_name: &str) {
        ::log::info!("Extraction complete: {}", package_name);
    }

    fn on_dependency_start(&self, dependency_name: &str, parent_package: &str) {
        ::log::info!("Installing dependency {} for {}", dependency_name, parent_package);
    }
}

//==================================================================================================
// Type Aliases
//==================================================================================================

///
/// # Description
///
/// Type alias for a shared progress callback.
///
pub type SharedProgress = Arc<dyn ProgressCallback>;

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    ///
    /// # Description
    ///
    /// Tests that NoOpProgress implements all trait methods without errors.
    ///
    #[test]
    fn test_no_op_progress() {
        let progress: NoOpProgress = NoOpProgress;
        progress.on_download_start("test", Some(1000));
        progress.on_download_complete("test");
        progress.on_extract_start("test");
        progress.on_extract_complete("test");
        progress.on_dependency_start("dep", "parent");
    }

    ///
    /// # Description
    ///
    /// Tests that LoggingProgress implements all trait methods without errors.
    ///
    #[test]
    fn test_logging_progress() {
        let progress: LoggingProgress = LoggingProgress;
        progress.on_download_start("test", Some(1000));
        progress.on_download_start("test", None);
        progress.on_download_complete("test");
        progress.on_extract_start("test");
        progress.on_extract_complete("test");
        progress.on_dependency_start("dep", "parent");
    }

    ///
    /// # Description
    ///
    /// Tests that SharedProgress can be created from NoOpProgress.
    ///
    #[test]
    fn test_shared_progress() {
        let progress: SharedProgress = Arc::new(NoOpProgress);
        progress.on_download_start("test", Some(100));
    }
}
