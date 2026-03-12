// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Overview
//!
//! The `nanvix-registry` library provides functionality for managing a local cache of Nanvix
//! release binaries downloaded from GitHub releases. It automatically downloads, extracts, and
//! caches binaries for different deployment types and target machines, supporting multiple
//! versions to coexist simultaneously. Additionally, it supports installing third-party packages
//! with automatic dependency resolution.
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
//! - Package installation with transitive dependency resolution.
//!
//! # Supported Packages
//!
//! The following packages can be installed using the registry:
//!
//! - `openblas`: OpenBLAS library for linear algebra operations.
//! - `openssl`: OpenSSL cryptographic library.
//! - `sqlite`: SQLite embedded database library.
//! - `zlib`: Zlib compression library.
//! - `quickjs`: QuickJS JavaScript engine.
//! - `cpython` (or `python`, `python3`): CPython interpreter.
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
//!         .get_cached_binary("microvm", "single-process", 128, "kernel.elf")
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
//! ## Installing Packages
//!
//! ```no_run
//! use nanvix_registry::Registry;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let registry: Registry = Registry::new(None);
//!
//!     // Install CPython and all its dependencies.
//!     // This will automatically download openssl, zlib, and other required packages.
//!     // Use `true` to fall back to latest version if compatible version not found.
//!     let install_path: String = registry
//!         .install("microvm", "single-process", 128, "python", true)
//!         .await?;
//!
//!     println!("Python installed to: {}", install_path);
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
//!         .get_cached_binary("hyperlight", "multi-process", 128, "kernel.elf")
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
//! - `package`: Defines package types and handles package downloads from GitHub.
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
//! multiple machine-deployment configurations and installed packages. Each release entry contains:
//! - The URL of the release tarball.
//! - The commit ID of the downloaded artifacts.
//!
//! Each package entry contains:
//! - The URL of the package tarball.
//! - The Nanvix commit ID the package is built for.
//!
//! The key format is `<machine>-<deployment>` for releases and `<machine>-<deployment>-<package>`
//! for packages. Multiple versions can coexist in separate subdirectories. The registry tracks
//! the most recent commit ID for each configuration, enabling the library to:
//! - Detect when new releases are available for specific configurations.
//! - Automatically download new releases while preserving older versions.
//! - Support side-by-side deployment of different machine-deployment combinations.
//! - Track installed packages and their versions.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
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
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Private Modules
//==================================================================================================

mod checksum;
mod deployment;
mod github_client;
mod machine;
mod metadata;
mod package;
mod progress;
mod rate_limiter;
mod release;
mod tarball;
mod tempfile;

//==================================================================================================
// Exports
//==================================================================================================

pub use crate::{
    deployment::Deployment,
    machine::Machine,
    package::Package,
    progress::{
        LoggingProgress,
        NoOpProgress,
        ProgressCallback,
        SharedProgress,
    },
};

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    metadata::ReleaseRegistry,
    package::{
        PackageDependencies,
        PackageRelease,
    },
    release::LatestRelease,
};
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    info,
    warn,
};
use ::std::{
    collections::HashSet,
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
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
/// Context for package installation operations.
///
/// This struct groups related parameters for the recursive package installation process,
/// reducing the number of function arguments and improving code maintainability.
///
struct InstallContext<'a> {
    /// Target machine type.
    machine: Machine,
    /// Deployment type.
    deployment: Deployment,
    /// Memory size in megabytes.
    memory_size_mb: u32,
    /// Nanvix commit ID.
    commit_id: &'a str,
    /// Cache directory path.
    cache_dir: &'a Path,
    /// Whether to fall back to latest release when compatible version is not found.
    use_latest_fallback: bool,
    /// Progress callback for reporting installation progress.
    progress: Arc<dyn ProgressCallback>,
}

impl<'a> InstallContext<'a> {
    /// Returns the target machine type.
    fn machine(&self) -> Machine {
        self.machine
    }

    /// Returns the deployment type.
    fn deployment(&self) -> Deployment {
        self.deployment
    }

    /// Returns the memory size in megabytes.
    fn memory_size_mb(&self) -> u32 {
        self.memory_size_mb
    }

    /// Returns the Nanvix commit ID.
    fn commit_id(&self) -> &str {
        self.commit_id
    }

    /// Returns the cache directory path.
    fn cache_dir(&self) -> &Path {
        self.cache_dir
    }

    /// Returns whether to fall back to latest release.
    fn use_latest_fallback(&self) -> bool {
        self.use_latest_fallback
    }

    /// Returns the progress callback.
    fn progress(&self) -> &dyn ProgressCallback {
        self.progress.as_ref()
    }
}

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
///         .get_cached_binary("microvm", "single-process", 128, "kernel.elf")
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
    /// - `memory_size_mb`: Memory size in megabytes for selecting the correct release archive.
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
    ///         .get_cached_binary("hyperlight", "multi-process", 128, "qjs")
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
        memory_size_mb: u32,
        binary_name: &str,
    ) -> Result<String> {
        // Use get_cached_artifact to search for the binary within the "bin" directory.
        self.get_cached_artifact(
            machine,
            deployment,
            memory_size_mb,
            binary_name,
            Some(BINARY_DIRECTORY_NAME),
        )
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
    /// - `memory_size_mb`: Memory size in megabytes for selecting the correct release archive.
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
    ///         .get_cached_artifact("hyperlight", "multi-process", 128, "config.json", None)
    ///         .await?;
    ///
    ///     // Search for a library file in a specific subdirectory.
    ///     let lib_path: String = registry
    ///         .get_cached_artifact("microvm", "single-process", 128, "libssl.so", Some("lib"))
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
        memory_size_mb: u32,
        artifact_name: &str,
        dir: Option<&str>,
    ) -> Result<String> {
        anyhow::ensure!(memory_size_mb > 0, "memory_size_mb must be greater than 0");

        let cache_dir: PathBuf = self.get_cache_dir().await?;

        // Convert machine from string representation.
        let machine: Machine = Machine::try_from(machine)?;

        // Convert deployment from string representation.
        let deployment: Deployment = Deployment::try_from(deployment)?;

        // Create release handle for checking latest release.
        let release: LatestRelease = LatestRelease::new(deployment, machine, memory_size_mb);

        // Get the latest release URL.
        let latest_url: String = release.get_url().await?;

        // Extract commit ID from URL (format: .../release-<commit_id>.tar.bz2).
        let commit_id: String = match Self::extract_commit_id(&latest_url) {
            Some(id) => {
                debug!("Extracted commit ID from URL: {}", id);
                id
            },
            None => {
                let reason: String = format!("Failed to extract commit ID from URL: {latest_url}");
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };

        // Construct the subdirectory name: <machine>-<deployment>-<memory_size>mb-<commit_id>.
        let subdir_name: String =
            format!("{}-{}-{}mb-{}", machine, deployment, memory_size_mb, commit_id);
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
            if let Some(cached_entry) = registry.get_release(machine, deployment, memory_size_mb) {
                if cached_entry.commit_id() != commit_id.as_str() {
                    info!(
                        "New release detected for {}-{}-{}mb (cached: {}, latest: {})",
                        machine,
                        deployment,
                        memory_size_mb,
                        cached_entry.commit_id(),
                        commit_id
                    );
                    true
                } else {
                    debug!(
                        "Using cached release for {}-{}-{}mb: {}",
                        machine,
                        deployment,
                        memory_size_mb,
                        cached_entry.commit_id()
                    );
                    false
                }
            } else {
                // Configuration not in registry, need to download.
                info!(
                    "Configuration {}-{}-{}mb not cached, downloading...",
                    machine, deployment, memory_size_mb
                );
                true
            };

        if needs_download {
            // Create the artifact cache directory.
            if let Err(error) = fs::create_dir_all(&artifact_cache_dir).await {
                let reason: String = format!("Failed to create artifact cache directory: {error}");
                error!("{reason}");
                anyhow::bail!(reason);
            }

            // Download and extract the release.
            let downloaded_url: String = release.download(&artifact_cache_dir).await?;

            // Update the registry with the new release.
            registry.set_release(machine, deployment, memory_size_mb, downloaded_url, commit_id);
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
    /// Installs a package and its dependencies for the current Nanvix release.
    ///
    /// This method first ensures the Nanvix release is cached, then downloads and installs the
    /// specified package and all its transitive dependencies. Packages are installed in the
    /// cache directory matching the Nanvix version.
    ///
    /// # Parameters
    ///
    /// - `machine`: Target machine type. Supported values:
    ///   - `"hyperlight"`: Hyperlight machine type.
    ///   - `"microvm"`: microvm machine type.
    /// - `deployment`: Deployment type. Supported values:
    ///   - `"single-process"`: Single-process deployment mode.
    ///   - `"multi-process"`: Multi-process deployment mode.
    /// - `memory_size_mb`: Memory size in megabytes for selecting the correct release archive.
    /// - `package_name`: Name of the package to install. Supported packages:
    ///   - `"openblas"`: OpenBLAS library.
    ///   - `"openssl"`: OpenSSL library.
    ///   - `"sqlite"`: SQLite library.
    ///   - `"zlib"`: Zlib compression library.
    ///   - `"quickjs"`: QuickJS JavaScript engine.
    ///   - `"cpython"` / `"python"` / `"python3"`: CPython interpreter.
    /// - `use_latest_fallback`: If `true`, falls back to the latest available package version
    ///   when a version compatible with the current Nanvix release is not found. If `false`,
    ///   returns an error when no compatible version exists.
    ///
    /// # Returns
    ///
    /// The absolute path to the directory where the package was installed.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The machine type is not recognized.
    /// - The deployment type is not recognized.
    /// - The package name is not recognized.
    /// - The GitHub API request fails.
    /// - The package tarball cannot be downloaded or extracted.
    /// - A dependency cannot be installed.
    /// - File system operations fail.
    /// - No compatible package version is found and `use_latest_fallback` is `false`.
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
    ///     // Install CPython and its dependencies (strict mode - requires compatible version).
    ///     let install_path: String = registry
    ///         .install("microvm", "single-process", 128, "python", false)
    ///         .await?;
    ///
    ///     // Install with fallback to latest if compatible version not found.
    ///     let install_path: String = registry
    ///         .install("microvm", "single-process", 128, "python", true)
    ///         .await?;
    ///
    ///     println!("Package installed to: {}", install_path);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    pub async fn install(
        &self,
        machine: &str,
        deployment: &str,
        memory_size_mb: u32,
        package_name: &str,
        use_latest_fallback: bool,
    ) -> Result<String> {
        self.install_with_progress(
            machine,
            deployment,
            memory_size_mb,
            package_name,
            use_latest_fallback,
            Arc::new(NoOpProgress),
        )
        .await
    }

    ///
    /// # Description
    ///
    /// Installs a package and its dependencies with progress reporting.
    ///
    /// This method is identical to [`install`](Self::install) but accepts a progress callback
    /// to receive updates during the installation process.
    ///
    /// # Parameters
    ///
    /// - `machine`: Target machine type (see [`install`](Self::install) for supported values).
    /// - `deployment`: Deployment type (see [`install`](Self::install) for supported values).
    /// - `memory_size_mb`: Memory size in megabytes for selecting the correct release archive.
    /// - `package_name`: Name of the package to install.
    /// - `use_latest_fallback`: If `true`, falls back to latest version when compatible not found.
    /// - `progress`: A shared progress callback to receive installation updates.
    ///
    /// # Returns
    ///
    /// The absolute path to the directory where the package was installed.
    ///
    /// # Errors
    ///
    /// Same error conditions as [`install`](Self::install).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nanvix_registry::{Registry, LoggingProgress};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let registry: Registry = Registry::new(None);
    ///     let progress = Arc::new(LoggingProgress);
    ///
    ///     let install_path: String = registry
    ///         .install_with_progress("microvm", "single-process", 128, "python", true, progress)
    ///         .await?;
    ///
    ///     println!("Package installed to: {}", install_path);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    pub async fn install_with_progress(
        &self,
        machine: &str,
        deployment: &str,
        memory_size_mb: u32,
        package_name: &str,
        use_latest_fallback: bool,
        progress: SharedProgress,
    ) -> Result<String> {
        anyhow::ensure!(memory_size_mb > 0, "memory_size_mb must be greater than 0");

        let cache_dir: PathBuf = self.get_cache_dir().await?;

        // Convert machine from string representation.
        let machine: Machine = Machine::try_from(machine)?;

        // Convert deployment from string representation.
        let deployment: Deployment = Deployment::try_from(deployment)?;

        // Convert package from string representation.
        let package: Package = Package::try_from(package_name)?;

        // First, ensure we have the Nanvix release cached to get the commit ID.
        let release: LatestRelease = LatestRelease::new(deployment, machine, memory_size_mb);
        let latest_url: String = release.get_url().await?;

        let commit_id: String = match Self::extract_commit_id(&latest_url) {
            Some(id) => {
                debug!("Nanvix commit ID: {}", id);
                id
            },
            None => {
                let reason: String = format!("Failed to extract commit ID from URL: {latest_url}");
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };

        // Load or create the release registry once at the start.
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
            info!("Creating a new registry...");
            ReleaseRegistry::new()
        };

        // Install the package (and dependencies) recursively.
        let context: InstallContext<'_> = InstallContext {
            machine,
            deployment,
            memory_size_mb,
            commit_id: &commit_id,
            cache_dir: &cache_dir,
            use_latest_fallback,
            progress,
        };
        let mut in_progress: HashSet<Package> = HashSet::new();
        self.install_package_recursive(&context, package, &mut registry, &mut in_progress)
            .await
    }

    ///
    /// # Description
    ///
    /// Internal helper method to recursively install a package and its dependencies.
    ///
    /// # Parameters
    ///
    /// - `context`: The installation context containing machine, deployment, commit ID,
    ///   cache directory, and fallback settings.
    /// - `package`: The package to install.
    /// - `registry`: Mutable reference to the release registry for tracking installations.
    /// - `in_progress`: Set of packages currently being installed, used to detect circular
    ///   dependencies.
    ///
    /// # Returns
    ///
    /// The absolute path to the directory where the package was installed.
    ///
    /// # Errors
    ///
    /// This function returns an error if the package or any dependency cannot be installed,
    /// or if a circular dependency is detected.
    ///
    async fn install_package_recursive(
        &self,
        context: &InstallContext<'_>,
        package: Package,
        registry: &mut ReleaseRegistry,
        in_progress: &mut HashSet<Package>,
    ) -> Result<String> {
        // Detect circular dependencies.
        if !in_progress.insert(package) {
            let reason: String = format!("Circular dependency detected for package {package}");
            error!("{reason}");
            anyhow::bail!(reason);
        }

        // Check if package is already installed for this configuration.
        if registry.is_package_installed(
            context.machine(),
            context.deployment(),
            package,
            context.commit_id(),
        ) {
            let subdir_name: String = format!(
                "{}-{}-{}mb-{}",
                context.machine(),
                context.deployment(),
                context.memory_size_mb(),
                context.commit_id()
            );
            let package_dir: PathBuf = context.cache_dir().join(&subdir_name);
            info!(
                "Package {} already installed for {}-{}-{}mb-{}",
                package,
                context.machine(),
                context.deployment(),
                context.memory_size_mb(),
                context.commit_id()
            );
            in_progress.remove(&package);
            return Ok(package_dir.to_string_lossy().to_string());
        }

        // Create package release handle.
        let package_release: PackageRelease = PackageRelease::new(
            package,
            context.deployment(),
            context.machine(),
            context.commit_id().to_string(),
            context.use_latest_fallback(),
        )?;

        // First, fetch and install dependencies.
        let dependencies: PackageDependencies = package_release.get_dependencies().await?;
        for dep_name in dependencies.dependencies() {
            let dep_package: Package = match Package::try_from(dep_name.as_str()) {
                Ok(p) => p,
                Err(error) => {
                    let reason: String = format!(
                        "Unknown dependency '{}' for package {}: {}",
                        dep_name, package, error
                    );
                    error!("{reason}");
                    anyhow::bail!(reason)
                },
            };

            // Notify progress callback about dependency installation.
            context
                .progress()
                .on_dependency_start(dep_name, package.as_str());

            info!("Installing dependency {} for package {}", dep_package, package);

            // Recursively install the dependency using Box::pin to avoid recursion issues.
            Box::pin(self.install_package_recursive(context, dep_package, registry, in_progress))
                .await?;
        }

        // Now install the package itself.
        let subdir_name: String = format!(
            "{}-{}-{}mb-{}",
            context.machine(),
            context.deployment(),
            context.memory_size_mb(),
            context.commit_id()
        );
        let package_dir: PathBuf = context.cache_dir().join(&subdir_name);

        // Create the package directory if it doesn't exist.
        if let Err(error) = fs::create_dir_all(&package_dir).await {
            let reason: String = format!("Failed to create package directory: {error}");
            error!("{reason}");
            anyhow::bail!(reason);
        }

        info!(
            "Installing package {} for {}-{}-{}mb-{}",
            package,
            context.machine(),
            context.deployment(),
            context.memory_size_mb(),
            context.commit_id()
        );

        // Notify progress callback about download start.
        context.progress().on_download_start(package.as_str(), None);

        // Download and extract the package.
        let downloaded_url: String = package_release.download(&package_dir).await?;

        // Notify progress callback about download completion.
        context.progress().on_download_complete(package.as_str());

        // Notify progress callback about extraction start.
        context.progress().on_extract_start(package.as_str());

        // Update and save the registry with the installed package.
        // We save after each package to persist partial progress on failure.
        registry.set_package(
            context.machine(),
            context.deployment(),
            package,
            downloaded_url,
            context.commit_id().to_string(),
        );
        registry.save(context.cache_dir()).await?;

        // Notify progress callback about extraction completion.
        context.progress().on_extract_complete(package.as_str());

        info!("Successfully installed package {} to {:?}", package, package_dir);

        in_progress.remove(&package);
        Ok(package_dir.to_string_lossy().to_string())
    }

    ///
    /// # Description
    ///
    /// Extracts the commit ID from a GitHub release URL.
    ///
    /// The commit ID is encoded in the release filename and uniquely identifies a specific
    /// build of Nanvix artifacts.
    ///
    /// Supported URL formats:
    /// - Legacy: `https://github.com/nanvix/nanvix/releases/download/latest/nanvix-hyperlight-multi-process-release-abc123def456.tar.bz2`
    /// - New:    `https://github.com/nanvix/nanvix/releases/download/latest/nanvix-hyperlight-multi-process-release-128mb-abc123def456.tar.bz2`
    ///
    /// The commit ID is always the last hyphen-delimited segment before the file extension.
    ///
    /// # Parameters
    ///
    /// - `url`: The GitHub release URL containing the commit ID.
    ///
    /// # Returns
    ///
    /// An `Option<String>` containing the commit ID if found, or `None` if the URL format
    /// is invalid.
    ///
    fn extract_commit_id(url: &str) -> Option<String> {
        // Extract the filename from the URL.
        let filename: &str = url.rsplit('/').next()?;

        // Strip known archive extensions.
        let stem: &str = filename
            .strip_suffix(".tar.bz2")
            .or_else(|| filename.strip_suffix(".tar.gz"))?;

        // Ensure the filename contains "release-" to validate the format.
        if !stem.contains("release-") {
            return None;
        }

        // The commit ID is the last hyphen-delimited segment of the stem.
        let commit_id: &str = stem.rsplit('-').next()?;

        // Validate that the commit ID is a non-empty hexadecimal string.
        if commit_id.is_empty() || !commit_id.chars().all(|c: char| c.is_ascii_hexdigit()) {
            None
        } else {
            Some(commit_id.to_string())
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
                    let reason: String = format!("Failed to spawn blocking task: {error}");
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
#[allow(clippy::expect_used)]
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

        let cache_dir: PathBuf = result.expect("failed");
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

        let cache_dir: PathBuf = registry.get_cache_dir().await.expect("failed");
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
                .expect("failed")
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
            .get_cached_binary("invalid-machine", "single-process", 128, "kernel.elf")
            .await;

        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
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
            .get_cached_binary("microvm", "invalid-deployment", 128, "kernel.elf")
            .await;

        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
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
            .get_cached_artifact("invalid-machine", "single-process", 128, "config.json", None)
            .await;

        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
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
            .get_cached_artifact("microvm", "invalid-deployment", 128, "config.json", None)
            .await;

        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
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
        fs::create_dir_all(&sub_dir).await.expect("failed");
        fs::write(&test_file, "test content").await.expect("failed");

        // Test that the artifact can be found.
        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "test-artifact.txt".to_string()).await;

        assert!(result.is_some());
        assert_eq!(result.expect("failed"), test_file);

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
            .get_cached_artifact("microvm", "single-process", 128, "nonexistent.txt", None)
            .await;
        assert!(result.is_err());

        // Test with Some custom directory - should also fail gracefully since no actual cache exists.
        let result: Result<String> = registry
            .get_cached_artifact("microvm", "single-process", 128, "nonexistent.txt", Some("lib"))
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
        fs::create_dir_all(&bin_dir).await.expect("failed");
        fs::create_dir_all(&lib_dir).await.expect("failed");
        fs::write(&root_artifact, "root config")
            .await
            .expect("failed");
        fs::write(&bin_artifact, "bin config")
            .await
            .expect("failed");
        fs::write(&lib_artifact, "lib config")
            .await
            .expect("failed");

        // Test that searching from cache root finds the root artifact.
        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "root-config.json".to_string()).await;
        assert!(result.is_some());
        assert_eq!(result.expect("failed"), root_artifact);

        // Test that searching from cache root finds artifacts in subdirectories.
        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "bin-config.json".to_string()).await;
        assert!(result.is_some());
        assert_eq!(result.expect("failed"), bin_artifact);

        let result: Option<PathBuf> =
            Registry::search_artifact(temp_dir.clone(), "lib-config.json".to_string()).await;
        assert!(result.is_some());
        assert_eq!(result.expect("failed"), lib_artifact);

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
        // Test valid URL with commit ID and memory size.
        let url: &str = "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-hyperlight-multi-process-release-128mb-abc123def456.tar.bz2";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_some());
        assert_eq!(commit_id.expect("failed"), "abc123def456");

        // Test another valid URL format with different memory size.
        let url: &str = "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-microvm-single-process-release-256mb-1a2b3c4d5e6f.tar.bz2";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_some());
        assert_eq!(commit_id.expect("failed"), "1a2b3c4d5e6f");

        // Test valid URL with .tar.gz extension.
        let url: &str = "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-hyperlight-multi-process-release-128mb-abc123def456.tar.gz";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_some());
        assert_eq!(commit_id.expect("failed"), "abc123def456");

        // Test URL without release prefix.
        let url: &str =
            "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-abc123def.tar.bz2";
        let commit_id: Option<String> = Registry::extract_commit_id(url);
        assert!(commit_id.is_none());

        // Test URL without file extension.
        let url: &str =
            "https://github.com/nanvix/nanvix/releases/download/latest/nanvix-release-abc123def";
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

    ///
    /// # Description
    ///
    /// Tests that install returns error for invalid machine type.
    ///
    #[tokio::test]
    async fn test_install_invalid_machine() {
        let registry: Registry = Registry::new(None);
        let result: Result<String> = registry
            .install("invalid-machine", "single-process", 128, "openssl", false)
            .await;
        assert!(result.is_err());
    }

    ///
    /// # Description
    ///
    /// Tests that install returns error for invalid deployment type.
    ///
    #[tokio::test]
    async fn test_install_invalid_deployment() {
        let registry: Registry = Registry::new(None);
        let result: Result<String> = registry
            .install("microvm", "invalid-deployment", 128, "openssl", false)
            .await;
        assert!(result.is_err());
    }

    ///
    /// # Description
    ///
    /// Tests that install returns error for invalid package name.
    ///
    #[tokio::test]
    async fn test_install_invalid_package() {
        let registry: Registry = Registry::new(None);
        let result: Result<String> = registry
            .install("microvm", "single-process", 128, "invalid-package", false)
            .await;
        assert!(result.is_err());
    }

    ///
    /// # Description
    ///
    /// Tests downloading and installing all packages.
    ///
    /// # Note
    ///
    /// This unit test is marked with `#[ignore]` because it performs actual network requests to
    /// GitHub and downloads multiple packages, which can be time-consuming and may not be suitable
    /// for regular test runs. It can be run manually when needed to verify the installation process
    /// end-to-end.
    ///
    #[tokio::test]
    #[ignore]
    async fn test_install_all_packages() {
        use ::tokio::fs;

        // Create a temporary directory for the test cache.
        let temp_dir: PathBuf =
            ::std::env::temp_dir().join("nanvix-registry-test-install-all-packages");

        // Clean up any existing test directory.
        let _ = fs::remove_dir_all(&temp_dir).await;

        // Run test logic and capture result to ensure cleanup runs even on failure.
        let test_result: Result<()> = async {
            // Create registry with custom cache directory.
            let registry: Registry = Registry::new(Some(temp_dir.clone()));

            // Attempt to install all packages.
            for package in Package::all() {
                let package_name: &str = package.as_str();

                // Attempt to install the package.
                let result: Result<String> = registry
                    .install("microvm", "single-process", 128, package_name, true)
                    .await;

                // Verify installation succeeded.
                let install_path: String = result?;
                anyhow::ensure!(
                    !install_path.is_empty(),
                    "Install path is empty for package: {}",
                    package_name
                );

                // Verify the install path exists.
                let path: PathBuf = PathBuf::from(&install_path);
                anyhow::ensure!(
                    fs::metadata(&path).await.is_ok(),
                    "Install path does not exist for package {}: {}",
                    package_name,
                    install_path
                );
            }

            Ok(())
        }
        .await;

        // Clean up regardless of test outcome.
        let _ = fs::remove_dir_all(&temp_dir).await;

        // Now assert the test result.
        assert!(test_result.is_ok(), "Test failed: {:?}", test_result.err());
    }
}
