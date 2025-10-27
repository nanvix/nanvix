// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Overview
//!
//! The `nanvix-registry` library provides functionality for managing a local cache of Nanvix
//! release binaries downloaded from GitHub releases. It automatically downloads, extracts, and
//! caches binaries for different deployment types and target machines.
//!
//! # Features
//!
//! - Downloads latest Nanvix releases from GitHub.
//! - Caches binaries locally in the user's cache directory.
//! - Supports multiple deployment types (`single-process`, `multi-process`).
//! - Supports multiple target machines (`hyperlight`, `microvm`).
//! - Automatic tarball extraction (supports `.tar.bz2` format).
//! - Cache management (automatic reuse and manual clearing).
//!
//! # Usage
//!
//! ```no_run
//! use nanvix_registry::Registry;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a new registry instance.
//!     let registry: Registry = Registry::new();
//!
//!     // Get a cached binary (downloads if not already cached).
//!     let binary_path: String = registry
//!         .get_cached_binary("microvm", "single-process", "kernel.elf")
//!         .await?;
//!
//!     println!("Binary path: {}", binary_path);
//!
//!     // Clear the cache when needed.
//!     registry.clear_cache().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! The library consists of the following internal modules:
//!
//! - `deployment`: Defines deployment types (`SingleProcess`, `MultiProcess`).
//! - `machine`: Defines target machine types (`Hyperlight`, `microvm`).
//! - `release`: Handles fetching and downloading releases from GitHub API.
//! - `tarball`: Provides tarball extraction functionality.
//! - `tempfile`: Manages temporary files with automatic cleanup.
//!
//! # Cache Location
//!
//! Binaries are cached in the user's cache directory under `nanvix-registry/bin/`.
//! The exact location depends on the operating system:
//!
//! - Linux: `~/.cache/nanvix-registry/bin/`
//! - macOS: `~/Library/Caches/nanvix-registry/bin/`
//! - Windows: `%LOCALAPPDATA%\nanvix-registry\bin\`

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]

//==================================================================================================
// Private Modules
//==================================================================================================

mod deployment;
mod machine;
mod metadata;
mod release;
mod tarball;
mod tempfile;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    deployment::Deployment,
    machine::Machine,
    metadata::ReleaseMetadata,
    release::LatestRelease,
};
use ::anyhow::Result;
use ::std::path::PathBuf;
use ::syslog::{
    debug,
    error,
    info,
};
use ::tokio::fs;

//==================================================================================================
// Constants
//==================================================================================================

/// Name for cache directory.
const CACHE_DIRECTORY_NAME: &str = "nanvix-registry";

/// Name for binary directory within the registry.
const BINARY_DIRECTORY_NAME: &str = "bin";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A registry for managing cached Nanvix release binaries.
///
/// This struct provides methods to download, cache, and retrieve Nanvix binaries from GitHub
/// releases. Binaries are cached in the user's cache directory and automatically reused on
/// subsequent requests.
///
/// # Examples
///
/// ```no_run
/// use nanvix_registry::Registry;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let registry: Registry = Registry::new();
///
///     // Get the kernel binary for microvm single-process deployment.
///     let kernel_path: String = registry
///         .get_cached_binary("microvm", "single-process", "kernel.elf")
///         .await?;
///
///     Ok(())
/// }
/// ```
///
pub struct Registry;

//==================================================================================================
// Implementations
//==================================================================================================

impl Registry {
    ///
    /// # Description
    ///
    /// Creates a new registry instance for managing cached binaries.
    ///
    /// # Returns
    ///
    /// A new `Registry` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use nanvix_registry::Registry;
    ///
    /// let registry: Registry = Registry::new();
    /// ```
    ///
    pub fn new() -> Self {
        Registry
    }

    ///
    /// # Description
    ///
    /// Retrieves the path to a cached binary, downloading and extracting it from GitHub releases
    /// if not already cached.
    ///
    /// This method first checks if the binary exists in the local cache. If found, it returns the
    /// path immediately. Otherwise, it downloads the latest release from GitHub, extracts the
    /// tarball, and caches the binaries for future use.
    ///
    /// # Parameters
    ///
    /// - `machine`: Target machine type. Supported values:
    ///   - `"hyperlight"`: Hyperlight machine type.
    ///   - `"microvm"`: microvm machine type.
    /// - `deployment`: Deployment type. Supported values:
    ///   - `"single-process"`: Single-process deployment mode.
    ///   - `"multi-process"`: Multi-process deployment mode.
    /// - `binary_name`: Name of the binary file (e.g., `"qjs"`, `"python3"`, `"kernel.elf"`).
    ///
    /// # Returns
    ///
    /// The absolute path to the cached binary as a `String`.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The machine type is not recognized.
    /// - The deployment type is not recognized.
    /// - The GitHub API request fails.
    /// - The release tarball cannot be downloaded or extracted.
    /// - The binary is not found in the extracted release.
    /// - File system operations fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nanvix_registry::Registry;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let registry: Registry = Registry::new();
    ///
    ///     // Get the QuickJS binary for hyperlight multi-process deployment.
    ///     let qjs_path: String = registry
    ///         .get_cached_binary("hyperlight", "multi-process", "qjs")
    ///         .await?;
    ///
    ///     println!("QuickJS binary: {}", qjs_path);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    pub async fn get_cached_binary(
        &self,
        machine: &str,
        deployment: &str,
        binary_name: &str,
    ) -> Result<String> {
        let cache_dir: PathBuf = Self::get_cache_dir().await?;
        let binary_path: PathBuf = cache_dir.join(BINARY_DIRECTORY_NAME).join(binary_name);

        // Convert machine from string representation.
        let machine: Machine = Machine::try_from(machine)?;

        // Convert deployment from string representation.
        let deployment: Deployment = Deployment::try_from(deployment)?;

        // Create release handle for checking latest release.
        let release: LatestRelease = LatestRelease::new(deployment, machine);

        // Get the latest release URL.
        let latest_url: String = release.get_url().await?;

        // Check if we have cached metadata.
        let metadata_exists: bool = ReleaseMetadata::exists(&cache_dir).await;

        let needs_download: bool = if metadata_exists {
            // Load cached metadata and compare URLs.
            match ReleaseMetadata::load(&cache_dir).await {
                Ok(cached_metadata) => {
                    if cached_metadata.url != latest_url {
                        info!(
                            "New release detected (cached: {}, latest: {})",
                            cached_metadata.url, latest_url
                        );
                        info!("Clearing old cache...");
                        // Clear the cache to download the new release.
                        self.clear_cache().await?;
                        true
                    } else {
                        // URLs match, check if binary exists.
                        if fs::metadata(&binary_path).await.is_ok() {
                            debug!("Using cached binary: {:?}", binary_path);
                            false
                        } else {
                            // Metadata exists but binary is missing, re-download.
                            info!("Binary missing from cache, re-downloading...");
                            true
                        }
                    }
                },
                Err(_) => {
                    // Failed to load metadata, download fresh.
                    info!("Failed to load metadata, downloading fresh release...");
                    true
                },
            }
        } else {
            // No metadata, need to download.
            info!("Binary not cached, downloading latest release...");
            true
        };

        if needs_download {
            // Download and extract the release.
            let downloaded_url: String = release.download(&cache_dir).await?;

            // Save the release metadata.
            let metadata: ReleaseMetadata = ReleaseMetadata::new(downloaded_url);
            metadata.save(&cache_dir).await?;

            // Verify binary now exists.
            if fs::metadata(&binary_path).await.is_err() {
                let reason: String = format!("Binary not found after download: {:?}", binary_path);
                error!("{reason}");
                anyhow::bail!(reason);
            }
        }

        Ok(binary_path.to_string_lossy().to_string())
    }

    ///
    /// # Description
    ///
    /// Clears the binary cache by removing the entire cache directory and all its contents.
    ///
    /// This method deletes the `nanvix-registry/` directory from the user's cache location,
    /// removing all cached binaries. The next call to `get_cached_binary()` will trigger a fresh
    /// download from GitHub.
    ///
    /// # Returns
    ///
    /// An empty tuple on success.
    ///
    /// # Errors
    ///
    /// This function returns an error if the cache directory cannot be removed due to file system
    /// permission issues or I/O errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nanvix_registry::Registry;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let registry: Registry = Registry::new();
    ///
    ///     // Clear all cached binaries.
    ///     registry.clear_cache().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    pub async fn clear_cache(&self) -> Result<()> {
        let cache_dir: PathBuf = Self::get_cache_dir().await?;
        if fs::metadata(&cache_dir).await.is_ok() {
            // Delete metadata first.
            ReleaseMetadata::delete(&cache_dir).await?;
            // Then remove the entire cache directory.
            if let Err(error) = fs::remove_dir_all(&cache_dir).await {
                let reason: String = format!("Failed to clear cache: {error}");
                error!("{reason}");
                anyhow::bail!(reason);
            }
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Retrieves the cache directory path, creating it if it doesn't exist.
    ///
    /// This method determines the user's cache directory using platform-specific conventions and
    /// appends `nanvix-registry/` as the cache subdirectory. If the directory doesn't exist, it is
    /// created along with any necessary parent directories.
    ///
    /// # Returns
    ///
    /// The absolute path to the cache directory.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The user's cache directory cannot be determined.
    /// - The blocking task for retrieving the cache directory fails.
    /// - The cache directory cannot be created due to permission issues or I/O errors.
    ///
    async fn get_cache_dir() -> Result<PathBuf> {
        // Get user's cache directory.
        let cache_dir: PathBuf = match tokio::task::spawn_blocking(dirs::cache_dir).await {
            Ok(Some(dir)) => dir.join(CACHE_DIRECTORY_NAME),
            Ok(None) => {
                let reason: &str = "could not get user's cache directory";
                error!("{reason}");
                anyhow::bail!(reason);
            },
            Err(error) => {
                let reason: String = format!("failed to spawn blocking task: {error}");
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };

        // Create cache directory if it doesn't exist.
        if let Err(error) = fs::create_dir_all(&cache_dir).await {
            let reason: &str = "could not create cache directory";
            error!("{reason}: {}", error);
            anyhow::bail!(reason);
        }

        Ok(cache_dir)
    }
}

impl Default for Registry {
    ///
    /// # Description
    ///
    /// Creates a default registry instance.
    ///
    /// # Returns
    ///
    /// A new `Registry` instance.
    ///
    fn default() -> Self {
        Self::new()
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    ///
    /// # Description
    ///
    /// Tests Registry creation with new().
    ///
    #[test]
    fn test_new() {
        let _registry: Registry = Registry::new();
    }

    ///
    /// # Description
    ///
    /// Tests Registry creation with default().
    ///
    #[test]
    fn test_default() {
        let _registry: Registry = Registry::default();
    }

    ///
    /// # Description
    ///
    /// Tests cache directory name constant.
    ///
    #[test]
    fn test_cache_directory_name() {
        assert_eq!(CACHE_DIRECTORY_NAME, "nanvix-registry");
    }

    ///
    /// # Description
    ///
    /// Tests binary directory name constant.
    ///
    #[test]
    fn test_binary_directory_name() {
        assert_eq!(BINARY_DIRECTORY_NAME, "bin");
    }

    ///
    /// # Description
    ///
    /// Tests that cache directory can be retrieved.
    ///
    #[tokio::test]
    async fn test_get_cache_dir() {
        let result: Result<PathBuf> = Registry::get_cache_dir().await;
        assert!(result.is_ok());

        let cache_dir: PathBuf = result.unwrap();
        assert!(cache_dir.to_string_lossy().contains("nanvix-registry"));
    }

    ///
    /// # Description
    ///
    /// Tests that clear_cache works when cache doesn't exist.
    ///
    #[tokio::test]
    async fn test_clear_cache_nonexistent() {
        let registry: Registry = Registry::new();
        let result: Result<()> = registry.clear_cache().await;
        assert!(result.is_ok());
    }

    ///
    /// # Description
    ///
    /// Tests that invalid machine type returns error.
    ///
    #[tokio::test]
    async fn test_invalid_machine() {
        let registry: Registry = Registry::new();
        let result: Result<String> = registry
            .get_cached_binary("invalid-machine", "single-process", "kernel.elf")
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown machine type"));
    }

    ///
    /// # Description
    ///
    /// Tests that invalid deployment type returns error.
    ///
    #[tokio::test]
    async fn test_invalid_deployment() {
        let registry: Registry = Registry::new();
        let result: Result<String> = registry
            .get_cached_binary("microvm", "invalid-deployment", "kernel.elf")
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown deployment type"));
    }

    ///
    /// # Description
    ///
    /// Tests binary path construction.
    ///
    #[test]
    fn test_binary_path_construction() {
        let cache_dir: PathBuf = PathBuf::from("/tmp/cache");
        let binary_path: PathBuf = cache_dir.join(BINARY_DIRECTORY_NAME).join("kernel.elf");

        assert!(binary_path.to_string_lossy().contains("bin"));
        assert!(binary_path.to_string_lossy().ends_with("kernel.elf"));
    }
}
