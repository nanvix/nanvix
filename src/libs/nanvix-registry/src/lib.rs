// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Overview
//!
//! The `nanvix-registry` library provides functionality for managing a local cache of Nanvix
//! release binaries downloaded from GitHub releases. It automatically downloads, extracts, and
//! caches binaries for different deployment types and target machines, supporting multiple
//! versions to coexist simultaneously.
//!
//! # Features
//!
//! - Downloads latest Nanvix releases from GitHub.
//! - Caches binaries locally in the user's cache directory.
//! - Organizes cached artifacts by commit ID in subdirectories following the pattern `<machine>-<deployment>-<commit_id>`.
//! - Supports multiple deployment types (`single-process`, `multi-process`).
//! - Supports multiple target machines (`hyperlight`, `microvm`).
//! - Automatic tarball extraction (supports `.tar.bz2` format).
//! - Cache management (automatic reuse and manual clearing).
//! - Tracks latest downloaded artifacts via release registry.
//! - Allows multiple versions of each machine-deployment configuration to coexist.
//!
//! # Usage
//!
//! ## Basic Usage (Default Cache Directory)
//!
//! ```no_run
//! use nanvix_registry::Registry;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a new registry instance with default cache directory.
//!     let registry: Registry = Registry::new(None);
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
//! ## Custom Cache Directory
//!
//! ```no_run
//! use nanvix_registry::Registry;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a registry with a custom cache directory.
//!     let cache_dir: PathBuf = PathBuf::from("/tmp/my-nanvix-cache");
//!     let registry: Registry = Registry::new(Some(cache_dir));
//!
//!     // Use the registry normally - it will use the custom directory.
//!     let binary_path: String = registry
//!         .get_cached_binary("hyperlight", "multi-process", "kernel.elf")
//!         .await?;
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
//! - `metadata`: Manages release registry tracking multiple machine-deployment configurations.
//! - `release`: Handles fetching and downloading releases from GitHub API.
//! - `tarball`: Provides tarball extraction functionality.
//! - `tempfile`: Manages temporary files with automatic cleanup.
//!
//! # Cache Location
//!
//! Binaries are cached in the user's cache directory under
//! `nanvix-registry/<machine>-<deployment>-<commit_id>/bin/`.
//! The exact location depends on the operating system:
//!
//! - Linux: `~/.cache/nanvix-registry/<machine>-<deployment>-<commit_id>/bin/`
//! - macOS: `~/Library/Caches/nanvix-registry/<machine>-<deployment>-<commit_id>/bin/`
//! - Windows: `%LOCALAPPDATA%\nanvix-registry\<machine>-<deployment>-<commit_id>\bin\`
//!
//! A custom cache directory can be specified when creating a `Registry` instance by passing
//! a `PathBuf` to `Registry::new()`. This is useful for testing or when you need to isolate
//! the cache from the default location.
//!
//! # Metadata
//!
//! The registry maintains a `release-metadata.json` file in the cache directory root that tracks
//! multiple machine-deployment configurations. Each entry in the registry contains:
//! - The URL of the release tarball.
//! - The commit ID of the downloaded artifacts.
//!
//! The key format is `<machine>-<deployment>` (e.g., "microvm-single-process"), and multiple
//! versions can coexist in separate subdirectories. The registry tracks the most recent commit ID
//! for each configuration, enabling the library to:
//! - Detect when new releases are available for specific configurations.
//! - Automatically download new releases while preserving older versions.
//! - Support side-by-side deployment of different machine-deployment combinations.

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
// Exports
//==================================================================================================

pub use crate::{
    deployment::Deployment,
    machine::Machine,
};

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    metadata::ReleaseRegistry,
    release::LatestRelease,
};
use ::anyhow::Result;
use ::std::path::PathBuf;
use ::syslog::{
    debug,
    error,
    info,
    warn,
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
///     let registry: Registry = Registry::new(None);
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
pub struct Registry {
    /// Optional custom cache directory path.
    cache_dir: Option<PathBuf>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Registry {
    ///
    /// # Description
    ///
    /// Creates a new registry instance for managing cached binaries.
    ///
    /// # Parameters
    ///
    /// - `cache_dir`: Optional custom cache directory path. If `None`, uses the system's default
    ///   cache directory.
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
    /// // Use default cache directory.
    /// let registry: Registry = Registry::new(None);
    ///
    /// // Use custom cache directory.
    /// let registry: Registry = Registry::new(Some("/tmp/my-cache".into()));
    /// ```
    ///
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        Registry { cache_dir }
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
    ///     let registry: Registry = Registry::new(None);
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
        // Use get_cached_artifact to search for the binary within the "bin" directory.
        self.get_cached_artifact(machine, deployment, binary_name, Some(BINARY_DIRECTORY_NAME))
            .await
    }

    ///
    /// # Description
    ///
    /// Searches for a cached artifact (file) in the registry and returns the first occurrence found.
    ///
    /// This method shares the same initialization logic as `get_cached_binary`, ensuring the cache
    /// is up-to-date before searching. It then performs an iterative depth-first search through the
    /// cache directory (or specified subdirectory) to find the first file matching the given name.
    ///
    /// # Parameters
    ///
    /// - `machine`: Target machine type. Supported values:
    ///   - `"hyperlight"`: Hyperlight machine type.
    ///   - `"microvm"`: microvm machine type.
    /// - `deployment`: Deployment type. Supported values:
    ///   - `"single-process"`: Single-process deployment mode.
    ///   - `"multi-process"`: Multi-process deployment mode.
    /// - `artifact_name`: Name of the artifact file to search for (e.g., `"config.json"`, `"lib.so"`).
    /// - `dir`: Optional directory path relative to the cache directory root where the artifact
    ///   should be searched. If `None`, searches from the cache directory root.
    ///   If specified, searches in `<cache_root>/<dir>/` instead.
    ///
    /// # Returns
    ///
    /// The absolute path to the first cached artifact found as a `String`.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The machine type is not recognized.
    /// - The deployment type is not recognized.
    /// - The GitHub API request fails.
    /// - The release tarball cannot be downloaded or extracted.
    /// - The artifact is not found in the cached release.
    /// - File system operations fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nanvix_registry::Registry;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let registry: Registry = Registry::new(None);
    ///
    ///     // Search for a configuration file from the cache directory root.
    ///     let config_path: String = registry
    ///         .get_cached_artifact("hyperlight", "multi-process", "config.json", None)
    ///         .await?;
    ///
    ///     // Search for a library file in a specific subdirectory.
    ///     let lib_path: String = registry
    ///         .get_cached_artifact("microvm", "single-process", "libssl.so", Some("lib"))
    ///         .await?;
    ///
    ///     println!("Configuration file: {}", config_path);
    ///     println!("Library file: {}", lib_path);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    pub async fn get_cached_artifact(
        &self,
        machine: &str,
        deployment: &str,
        artifact_name: &str,
        dir: Option<&str>,
    ) -> Result<String> {
        let cache_dir: PathBuf = self.get_cache_dir().await?;

        // Convert machine from string representation.
        let machine: Machine = Machine::try_from(machine)?;

        // Convert deployment from string representation.
        let deployment: Deployment = Deployment::try_from(deployment)?;

        // Create release handle for checking latest release.
        let release: LatestRelease = LatestRelease::new(deployment, machine);

        // Get the latest release URL.
        let latest_url: String = release.get_url().await?;

        // Extract commit ID from URL (format: .../release-<commit_id>.tar.bz2).
        let commit_id: String = match Self::extract_commit_id(&latest_url) {
            Some(id) => {
                debug!("Extracted commit ID from URL: {}", id);
                id
            },
            None => {
                let reason: String =
                    format!("Failed to extract commit ID from URL: {}", latest_url);
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };

        // Construct the subdirectory name: <machine>-<deployment>-<commit_id>.
        let subdir_name: String = format!("{}-{}-{}", machine, deployment, commit_id);
        let artifact_cache_dir: PathBuf = cache_dir.join(&subdir_name);

        // Load or create the release registry.
        let mut registry: ReleaseRegistry = if ReleaseRegistry::exists(&cache_dir).await {
            match ReleaseRegistry::load(&cache_dir).await {
                Ok(reg) => reg,
                Err(error) => {
                    let reason: String = format!("Failed to load registry: {error}");
                    error!("{reason}");
                    anyhow::bail!(reason)
                },
            }
        } else {
            // No registry exists, create a new one.
            info!("Creating a new registry...");
            ReleaseRegistry::new()
        };

        // Check if we need to download this specific configuration.
        let needs_download: bool =
            if let Some(cached_entry) = registry.get_release(machine, deployment) {
                if cached_entry.commit_id() != commit_id.as_str() {
                    info!(
                        "New release detected for {}-{} (cached: {}, latest: {})",
                        machine,
                        deployment,
                        cached_entry.commit_id(),
                        commit_id
                    );
                    true
                } else {
                    debug!(
                        "Using cached release for {}-{}: {}",
                        machine,
                        deployment,
                        cached_entry.commit_id()
                    );
                    false
                }
            } else {
                // Configuration not in registry, need to download.
                info!("Configuration {}-{} not cached, downloading...", machine, deployment);
                true
            };

        if needs_download {
            // Create the artifact cache directory.
            if let Err(error) = fs::create_dir_all(&artifact_cache_dir).await {
                let reason: String =
                    format!("Failed to create artifact cache directory: {}", error);
                error!("{reason}");
                anyhow::bail!(reason);
            }

            // Download and extract the release.
            let downloaded_url: String = release.download(&artifact_cache_dir).await?;

            // Update the registry with the new release.
            registry.set_release(machine, deployment, downloaded_url, commit_id);
            registry.save(&cache_dir).await?;
        }

        // Now search for the artifact in the specified directory.
        let search_dir: PathBuf = match dir {
            Some(custom_dir) => artifact_cache_dir.join(custom_dir),
            None => artifact_cache_dir.clone(),
        };
        match Self::search_artifact(search_dir, artifact_name.to_string()).await {
            Some(artifact_path) => {
                debug!("Found artifact: {:?}", artifact_path);
                Ok(artifact_path.to_string_lossy().to_string())
            },
            None => {
                let reason: String =
                    format!("Artifact '{}' not found in cached release", artifact_name);
                error!("{reason}");
                anyhow::bail!(reason);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Extracts the build identifier from a GitHub release URL.
    ///
    /// The build identifier is a numeric value (such as a workflow run ID or timestamp) encoded in
    /// the release filename. It uniquely identifies a specific build of Nanvix artifacts.
    ///
    /// Example URL format:
    /// `https://github.com/nanvix/nanvix/releases/download/latest/nanvix-hyperlight-multi-process-release-19417333438.tar.bz2`
    ///
    /// This method parses the filename to extract the numeric identifier between "release-" and
    /// the file extension (e.g., `19417333438` from the example above).
    ///
    /// # Parameters
    ///
    /// - `url`: The GitHub release URL containing the build identifier.
    ///
    /// # Returns
    ///
    /// An `Option<String>` containing the build identifier if found, or `None` if the URL format
    /// is invalid.
    ///
    fn extract_commit_id(url: &str) -> Option<String> {
        // Extract the filename from the URL.
        let filename: &str = url.rsplit('/').next()?;

        // Find "release-" prefix.
        let release_prefix: &str = "release-";
        let start_idx: usize = filename.find(release_prefix)? + release_prefix.len();

        // Find the first dot after "release-" to identify start of file extension.
        let remaining: &str = &filename[start_idx..];
        let end_idx: usize = start_idx + remaining.find('.')?;

        // Extract and validate the commit ID.
        if start_idx < end_idx {
            let commit_id: &str = &filename[start_idx..end_idx];
            Some(commit_id.to_string())
        } else {
            None
        }
    }

    ///
    /// # Description
    ///
    /// Searches for an artifact file in the given directory tree.
    ///
    /// This helper method performs an iterative depth-first search through the directory tree to
    /// find the first file matching the given name. The implementation uses a stack-based approach
    /// to avoid recursion overhead and potential stack overflow issues.
    ///
    /// # Parameters
    ///
    /// - `dir`: The directory path to search in.
    /// - `artifact_name`: The name of the artifact file to search for.
    ///
    /// # Returns
    ///
    /// An `Option<PathBuf>` containing the path to the first matching artifact, or `None` if not found.
    ///
    async fn search_artifact(dir: PathBuf, artifact_name: String) -> Option<PathBuf> {
        let mut stack: Vec<PathBuf> = vec![dir];

        while let Some(current_dir) = stack.pop() {
            let mut read_dir = match fs::read_dir(&current_dir).await {
                Ok(read_dir) => read_dir,
                Err(error) => {
                    // Could not read directory, skip it.
                    warn!("failed to read '{current_dir:?}': {error}");
                    continue;
                },
            };

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path: PathBuf = entry.path();

                match fs::metadata(&path).await {
                    Ok(metadata) => {
                        if metadata.is_file() {
                            if let Some(file_name) = path.file_name() {
                                if file_name == artifact_name.as_str() {
                                    return Some(path);
                                }
                            }
                        } else if metadata.is_dir() {
                            // Add subdirectory to stack for later processing.
                            stack.push(path);
                        }
                    },
                    Err(error) => {
                        // Could not get metadata, skip this entry.
                        warn!("failed to read '{path:?}': {error}");
                        continue;
                    },
                }
            }
        }

        None
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
    ///     let registry: Registry = Registry::new(None);
    ///
    ///     // Clear all cached binaries.
    ///     registry.clear_cache().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    pub async fn clear_cache(&self) -> Result<()> {
        let cache_dir: PathBuf = self.get_cache_dir().await?;
        if fs::metadata(&cache_dir).await.is_ok() {
            // Delete registry first.
            ReleaseRegistry::delete(&cache_dir).await?;
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
    /// appends `nanvix-registry/` as the cache subdirectory. If a custom cache directory was
    /// provided during construction, it uses that instead. If the directory doesn't exist, it is
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
    async fn get_cache_dir(&self) -> Result<PathBuf> {
        // Get cache directory from custom path or user's cache directory.
        let cache_dir: PathBuf = match &self.cache_dir {
            Some(custom_dir) => custom_dir.clone(),
            None => match tokio::task::spawn_blocking(dirs::cache_dir).await {
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
        Self::new(None)
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
        let _registry: Registry = Registry::new(None);
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
        let registry: Registry = Registry::new(None);
        let result: Result<PathBuf> = registry.get_cache_dir().await;
        assert!(result.is_ok());

        let cache_dir: PathBuf = result.unwrap();
        assert!(cache_dir.to_string_lossy().contains("nanvix-registry"));
    }

    ///
    /// # Description
    ///
    /// Tests creating a Registry with a custom cache directory.
    ///
    #[tokio::test]
    async fn test_custom_cache_directory() {
        let custom_dir: PathBuf = ::std::env::temp_dir().join("nanvix-test-custom-cache");
        let registry: Registry = Registry::new(Some(custom_dir.clone()));

        let cache_dir: PathBuf = registry.get_cache_dir().await.unwrap();
        assert_eq!(cache_dir, custom_dir);

        // Cleanup
        let _ = ::tokio::fs::remove_dir_all(&custom_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests that clear_cache works when cache doesn't exist.
    ///
    #[tokio::test]
    async fn test_clear_cache_nonexistent() {
        use ::tokio::fs;

        // Create a unique temporary directory to avoid conflicts in NFS environments.
        let temp_dir: PathBuf = ::std::env::temp_dir().join(format!(
            "nanvix-registry-clear-test-{}-{}",
            ::std::process::id(),
            ::std::time::SystemTime::now()
                .duration_since(::std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        // Ensure the directory doesn't exist before the test.
        let _ = fs::remove_dir_all(&temp_dir).await;

        // Create a registry with the custom cache directory.
        let registry: Registry = Registry::new(Some(temp_dir.clone()));
        let result: Result<()> = registry.clear_cache().await;
        assert!(result.is_ok());

        // Clean up.
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests that invalid machine type returns error.
    ///
    #[tokio::test]
    async fn test_invalid_machine() {
        let registry: Registry = Registry::new(None);
        let result = registry
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
        let registry: Registry = Registry::new(None);
        let result = registry
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

    ///
    /// # Description
    ///
    /// Tests that invalid machine type returns error for get_cached_artifact.
    ///
    #[tokio::test]
    async fn test_get_cached_artifact_invalid_machine() {
        let registry: Registry = Registry::new(None);
        let result: Result<String> = registry
            .get_cached_artifact("invalid-machine", "single-process", "config.json", None)
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
    /// Tests that invalid deployment type returns error for get_cached_artifact.
    ///
    #[tokio::test]
    async fn test_get_cached_artifact_invalid_deployment() {
        let registry: Registry = Registry::new(None);
        let result: Result<String> = registry
            .get_cached_artifact("microvm", "invalid-deployment", "config.json", None)
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
    /// Tests artifact search functionality with a temporary directory structure.
    ///
    #[tokio::test]
    async fn test_search_artifact() {
        use ::tokio::fs;

        // Create a temporary directory structure for testing.
        let temp_dir: PathBuf = ::std::env::temp_dir().join("nanvix-registry-test");
        let sub_dir: PathBuf = temp_dir.join("subdir");
        let test_file: PathBuf = sub_dir.join("test-artifact.txt");

        // Clean up any existing test directory.
        let _ = fs::remove_dir_all(&temp_dir).await;

        // Create directory structure.
        fs::create_dir_all(&sub_dir).await.unwrap();
        fs::write(&test_file, "test content").await.unwrap();

        // Test that the artifact can be found.
        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "test-artifact.txt".to_string()).await;

        assert!(result.is_some());
        assert_eq!(result.unwrap(), test_file);

        // Test that non-existent artifact returns None.
        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "non-existent.txt".to_string()).await;

        assert!(result.is_none());

        // Clean up.
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests that custom directory parameter works correctly.
    ///
    #[tokio::test]
    async fn test_get_cached_artifact_custom_directory() {
        let registry: Registry = Registry::new(None);

        // Test with None (searches from cache root) - should fail gracefully since no actual cache exists.
        let result: Result<String> = registry
            .get_cached_artifact("microvm", "single-process", "nonexistent.txt", None)
            .await;
        assert!(result.is_err());

        // Test with Some custom directory - should also fail gracefully since no actual cache exists.
        let result: Result<String> = registry
            .get_cached_artifact("microvm", "single-process", "nonexistent.txt", Some("lib"))
            .await;
        assert!(result.is_err());
    }

    ///
    /// # Description
    ///
    /// Tests that search starts from cache root when no directory is specified.
    ///
    #[tokio::test]
    async fn test_get_cached_artifact_searches_from_cache_root() {
        use ::tokio::fs;

        // Create a temporary directory structure for testing.
        let temp_dir: PathBuf = ::std::env::temp_dir().join("nanvix-registry-cache-root-test");
        let bin_dir: PathBuf = temp_dir.join("bin");
        let lib_dir: PathBuf = temp_dir.join("lib");
        let root_artifact: PathBuf = temp_dir.join("root-config.json");
        let bin_artifact: PathBuf = bin_dir.join("bin-config.json");
        let lib_artifact: PathBuf = lib_dir.join("lib-config.json");

        // Clean up any existing test directory.
        let _ = fs::remove_dir_all(&temp_dir).await;

        // Create directory structure.
        fs::create_dir_all(&bin_dir).await.unwrap();
        fs::create_dir_all(&lib_dir).await.unwrap();
        fs::write(&root_artifact, "root config").await.unwrap();
        fs::write(&bin_artifact, "bin config").await.unwrap();
        fs::write(&lib_artifact, "lib config").await.unwrap();

        // Test that searching from cache root finds the root artifact.
        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "root-config.json".to_string()).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), root_artifact);

        // Test that searching from cache root finds artifacts in subdirectories.
        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "bin-config.json".to_string()).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), bin_artifact);

        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "lib-config.json".to_string()).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), lib_artifact);

        // Clean up.
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests commit ID extraction from GitHub release URLs.
    ///
    #[test]
    fn test_extract_commit_id() {
        // Test valid URL with commit ID.
        let url: &str = "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-hyperlight-multi-process-release-19417333438.tar.bz2";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_some());
        assert_eq!(commit_id.unwrap(), "19417333438");

        // Test another valid URL format.
        let url: &str = "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-microvm-single-process-release-12345678.tar.bz2";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_some());
        assert_eq!(commit_id.unwrap(), "12345678");

        // Test URL without release prefix.
        let url: &str =
            "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-12345678.tar.bz2";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_none());

        // Test URL without file extension.
        let url: &str =
            "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-release-12345678";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_none());

        // Test empty string.
        let url: &str = "";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_none());

        // Test malformed URL.
        let url: &str = "not-a-valid-url";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_none());
    }
}
