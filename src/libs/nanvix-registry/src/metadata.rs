// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    deployment::Deployment,
    machine::Machine,
    package::Package,
};
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    info,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::{
    collections::HashMap,
    path::{
        Path,
        PathBuf,
    },
};
use ::tokio::fs;

//==================================================================================================
// Constants
//==================================================================================================

/// Name of the metadata file.
const METADATA_FILE_NAME: &str = "release-metadata.json";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Metadata about a cached release for a specific machine-deployment configuration.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReleaseEntry {
    /// URL of the release tarball.
    url: String,
    /// Commit ID extracted from the release URL.
    commit_id: String,
}

///
/// # Description
///
/// Metadata about an installed package for a specific machine-deployment configuration.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PackageEntry {
    /// URL of the package tarball.
    url: String,
    /// Nanvix commit ID this package is built for.
    nanvix_commit_id: String,
}

///
/// # Description
///
/// Registry metadata tracking multiple machine-deployment configurations.
///
/// This structure maintains a map where keys are in the format "<machine>-<deployment>"
/// and values are `ReleaseEntry` instances containing the URL and commit ID of the
/// most recent release for that configuration.
///
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ReleaseRegistry {
    /// Map of machine-deployment configurations to their release entries.
    /// Key format: "<machine>-<deployment>" (e.g., "microvm-single-process").
    releases: HashMap<String, ReleaseEntry>,
    /// Map of installed packages to their entries.
    /// Key format: "<machine>-<deployment>-<package>" (e.g., "microvm-single-process-openssl").
    #[serde(default)]
    packages: HashMap<String, PackageEntry>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ReleaseEntry {
    ///
    /// # Description
    ///
    /// Creates a new release entry instance.
    ///
    /// # Parameters
    ///
    /// - `url`: The URL of the release tarball.
    /// - `commit_id`: The commit ID extracted from the release URL.
    ///
    /// # Returns
    ///
    /// A new `ReleaseEntry` instance.
    ///
    pub(crate) fn new(url: String, commit_id: String) -> Self {
        Self { url, commit_id }
    }

    ///
    /// # Description
    ///
    /// Returns the commit ID extracted from the release URL.
    ///
    /// # Returns
    ///
    /// A reference to the commit ID string.
    ///
    pub(crate) fn commit_id(&self) -> &str {
        &self.commit_id
    }

    ///
    /// # Description
    ///
    /// Returns the URL of the release tarball.
    ///
    /// # Returns
    ///
    /// A reference to the URL string.
    ///
    #[cfg(test)]
    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}

impl PackageEntry {
    ///
    /// # Description
    ///
    /// Creates a new package entry instance.
    ///
    /// # Parameters
    ///
    /// - `url`: The URL of the package tarball.
    /// - `nanvix_commit_id`: The Nanvix commit ID this package is built for.
    ///
    /// # Returns
    ///
    /// A new `PackageEntry` instance.
    ///
    pub(crate) fn new(url: String, nanvix_commit_id: String) -> Self {
        Self {
            url,
            nanvix_commit_id,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the Nanvix commit ID this package is built for.
    ///
    /// # Returns
    ///
    /// A reference to the Nanvix commit ID string.
    ///
    pub(crate) fn nanvix_commit_id(&self) -> &str {
        &self.nanvix_commit_id
    }

    ///
    /// # Description
    ///
    /// Returns the URL of the package tarball.
    ///
    /// # Returns
    ///
    /// A reference to the URL string.
    ///
    #[cfg(test)]
    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}

impl ReleaseRegistry {
    ///
    /// # Description
    ///
    /// Creates a new empty release registry.
    ///
    /// # Returns
    ///
    /// A new `ReleaseRegistry` instance.
    ///
    pub(crate) fn new() -> Self {
        Self {
            releases: HashMap::new(),
            packages: HashMap::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Adds or updates a release entry for a specific machine-deployment configuration.
    ///
    /// # Parameters
    ///
    /// - `machine`: The target machine type.
    /// - `deployment`: The deployment type.
    /// - `url`: The URL of the release tarball.
    /// - `commit_id`: The commit ID extracted from the release URL.
    ///
    pub(crate) fn set_release(
        &mut self,
        machine: Machine,
        deployment: Deployment,
        memory_size_mb: u32,
        url: String,
        commit_id: String,
    ) {
        let key: String = format!("{}-{}-{}mb", machine, deployment, memory_size_mb);
        let entry: ReleaseEntry = ReleaseEntry::new(url, commit_id);
        self.releases.insert(key, entry);
    }

    ///
    /// # Description
    ///
    /// Gets the release entry for a specific machine-deployment configuration.
    ///
    /// # Parameters
    ///
    /// - `machine`: The target machine type.
    /// - `deployment`: The deployment type.
    ///
    /// # Returns
    ///
    /// An `Option<&ReleaseEntry>` containing the release entry if found, or `None` otherwise.
    ///
    pub(crate) fn get_release(
        &self,
        machine: Machine,
        deployment: Deployment,
        memory_size_mb: u32,
    ) -> Option<&ReleaseEntry> {
        let key: String = format!("{}-{}-{}mb", machine, deployment, memory_size_mb);
        self.releases.get(&key)
    }

    ///
    /// # Description
    ///
    /// Returns the number of release entries in the registry.
    ///
    /// # Returns
    ///
    /// The number of release entries.
    ///
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.releases.len()
    }

    ///
    /// # Description
    ///
    /// Checks if the registry is empty.
    ///
    /// # Returns
    ///
    /// `true` if the registry contains no entries, `false` otherwise.
    ///
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.releases.is_empty()
    }

    ///
    /// # Description
    ///
    /// Adds or updates a package entry for a specific machine-deployment-package configuration.
    ///
    /// # Parameters
    ///
    /// - `machine`: The target machine type.
    /// - `deployment`: The deployment type.
    /// - `package`: The package being installed.
    /// - `url`: The URL of the package tarball.
    /// - `nanvix_commit_id`: The Nanvix commit ID this package is built for.
    ///
    pub(crate) fn set_package(
        &mut self,
        machine: Machine,
        deployment: Deployment,
        package: Package,
        url: String,
        nanvix_commit_id: String,
    ) {
        let key: String = format!("{}-{}-{}", machine, deployment, package);
        let entry: PackageEntry = PackageEntry::new(url, nanvix_commit_id);
        self.packages.insert(key, entry);
    }

    ///
    /// # Description
    ///
    /// Gets the package entry for a specific machine-deployment-package configuration.
    ///
    /// # Parameters
    ///
    /// - `machine`: The target machine type.
    /// - `deployment`: The deployment type.
    /// - `package`: The package to look up.
    ///
    /// # Returns
    ///
    /// An `Option<&PackageEntry>` containing the package entry if found, or `None` otherwise.
    ///
    pub(crate) fn get_package(
        &self,
        machine: Machine,
        deployment: Deployment,
        package: Package,
    ) -> Option<&PackageEntry> {
        let key: String = format!("{}-{}-{}", machine, deployment, package);
        self.packages.get(&key)
    }

    ///
    /// # Description
    ///
    /// Checks whether a package is installed for a specific machine-deployment configuration
    /// and matches the given Nanvix commit ID.
    ///
    /// # Parameters
    ///
    /// - `machine`: The target machine type.
    /// - `deployment`: The deployment type.
    /// - `package`: The package to check.
    /// - `nanvix_commit_id`: The Nanvix commit ID to match.
    ///
    /// # Returns
    ///
    /// `true` if the package is installed and matches the commit ID, `false` otherwise.
    ///
    pub(crate) fn is_package_installed(
        &self,
        machine: Machine,
        deployment: Deployment,
        package: Package,
        nanvix_commit_id: &str,
    ) -> bool {
        if let Some(entry) = self.get_package(machine, deployment, package) {
            entry.nanvix_commit_id() == nanvix_commit_id
        } else {
            false
        }
    }

    ///
    /// # Description
    ///
    /// Saves the release registry to a file in the specified directory.
    ///
    /// # Parameters
    ///
    /// - `cache_dir`: The directory where the metadata file will be saved.
    ///
    /// # Returns
    ///
    /// On success, this function returns an empty tuple. On failure, it returns an object that
    /// describes the error.
    ///
    pub(crate) async fn save(&self, cache_dir: &Path) -> Result<()> {
        let metadata_path: PathBuf = cache_dir.join(METADATA_FILE_NAME);
        let json: String = match serde_json::to_string_pretty(self) {
            Ok(json) => json,
            Err(error) => {
                let reason: String = format!("Failed to serialize registry: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        if let Err(error) = fs::write(&metadata_path, json).await {
            let reason: String = format!("Failed to write registry file: {error}");
            error!("{reason}");
            anyhow::bail!(reason)
        }

        debug!("Saved release registry to: {:?}", metadata_path);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Loads release registry from a file in the specified directory.
    ///
    /// # Parameters
    ///
    /// - `cache_dir`: The directory where the metadata file is located.
    ///
    /// # Returns
    ///
    /// On success, this function returns the loaded `ReleaseRegistry`. On failure, it returns an
    /// object that describes the error.
    ///
    pub(crate) async fn load(cache_dir: &Path) -> Result<Self> {
        let metadata_path: PathBuf = cache_dir.join(METADATA_FILE_NAME);

        // Check if metadata file exists.
        if fs::metadata(&metadata_path).await.is_err() {
            let reason: String = "Registry file not found".to_string();
            debug!("{reason}");
            anyhow::bail!(reason)
        }

        // Read metadata file.
        let json: String = match fs::read_to_string(&metadata_path).await {
            Ok(content) => content,
            Err(error) => {
                let reason: String = format!("Failed to read registry file: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Deserialize registry.
        let registry: ReleaseRegistry = match serde_json::from_str(&json) {
            Ok(registry) => registry,
            Err(error) => {
                let reason: String = format!("Failed to deserialize registry: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        debug!("Loaded release registry from: {:?}", metadata_path);
        Ok(registry)
    }

    ///
    /// # Description
    ///
    /// Checks if metadata exists in the specified directory.
    ///
    /// # Parameters
    ///
    /// - `cache_dir`: The directory to check for metadata.
    ///
    /// # Returns
    ///
    /// `true` if metadata exists, `false` otherwise.
    ///
    pub(crate) async fn exists(cache_dir: &Path) -> bool {
        let metadata_path: PathBuf = cache_dir.join(METADATA_FILE_NAME);
        fs::metadata(&metadata_path).await.is_ok()
    }

    ///
    /// # Description
    ///
    /// Deletes the metadata file from the specified directory.
    ///
    /// # Parameters
    ///
    /// - `cache_dir`: The directory where the metadata file is located.
    ///
    /// # Returns
    ///
    /// On success, this function returns an empty tuple. On failure, it returns an object that
    /// describes the error.
    ///
    pub(crate) async fn delete(cache_dir: &Path) -> Result<()> {
        let metadata_path: PathBuf = cache_dir.join(METADATA_FILE_NAME);

        if fs::metadata(&metadata_path).await.is_ok() {
            if let Err(error) = fs::remove_file(&metadata_path).await {
                let reason: String = format!("Failed to delete metadata file: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            }
            info!("Deleted release metadata from: {:?}", metadata_path);
        }

        Ok(())
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
    /// Tests release entry creation.
    ///
    #[test]
    fn test_entry_new() {
        let url: String = "https://github.com/test/release.tar.bz2".to_string();
        let commit_id: String = "abc123def456".to_string();
        let entry: ReleaseEntry = ReleaseEntry::new(url.clone(), commit_id.clone());
        assert_eq!(entry.url(), url);
        assert_eq!(entry.commit_id(), commit_id);
    }

    ///
    /// # Description
    ///
    /// Tests registry creation.
    ///
    #[test]
    fn test_registry_new() {
        let registry: ReleaseRegistry = ReleaseRegistry::new();
        assert!(registry.is_empty());
    }

    ///
    /// # Description
    ///
    /// Tests setting and getting release entries.
    ///
    #[test]
    fn test_set_and_get_release() {
        let mut registry: ReleaseRegistry = ReleaseRegistry::new();

        let url: String = "https://test.com/file.tar.bz2".to_string();
        let commit_id: String = "abc123def456".to_string();

        // Set a release.
        registry.set_release(
            Machine::Microvm,
            Deployment::SingleProcess,
            128,
            url.clone(),
            commit_id.clone(),
        );

        // Get the release.
        let entry: Option<&ReleaseEntry> =
            registry.get_release(Machine::Microvm, Deployment::SingleProcess, 128);
        assert!(entry.is_some());

        let entry: &ReleaseEntry = entry.expect("failed");
        assert_eq!(entry.url(), url);
        assert_eq!(entry.commit_id(), commit_id);
    }

    ///
    /// # Description
    ///
    /// Tests multiple configurations in registry.
    ///
    #[test]
    fn test_multiple_configurations() {
        let mut registry: ReleaseRegistry = ReleaseRegistry::new();

        // Add multiple configurations.
        registry.set_release(
            Machine::Microvm,
            Deployment::SingleProcess,
            128,
            "https://test.com/microvm-sp.tar.bz2".to_string(),
            "abc111def".to_string(),
        );

        registry.set_release(
            Machine::Microvm,
            Deployment::MultiProcess,
            128,
            "https://test.com/microvm-mp.tar.bz2".to_string(),
            "abc222def".to_string(),
        );

        registry.set_release(
            Machine::Hyperlight,
            Deployment::SingleProcess,
            128,
            "https://test.com/hyperlight-sp.tar.bz2".to_string(),
            "abc333def".to_string(),
        );

        // The number of entries is checked indirectly by verifying each entry exists below.
        assert_eq!(registry.len(), 3);

        // Verify each entry.
        let entry: &ReleaseEntry = registry
            .get_release(Machine::Microvm, Deployment::SingleProcess, 128)
            .expect("failed");
        assert_eq!(entry.commit_id(), "abc111def");

        let entry: &ReleaseEntry = registry
            .get_release(Machine::Microvm, Deployment::MultiProcess, 128)
            .expect("failed");
        assert_eq!(entry.commit_id(), "abc222def");

        let entry: &ReleaseEntry = registry
            .get_release(Machine::Hyperlight, Deployment::SingleProcess, 128)
            .expect("failed");
        assert_eq!(entry.commit_id(), "abc333def");
    }

    ///
    /// # Description
    ///
    /// Tests updating an existing configuration.
    ///
    #[test]
    fn test_update_existing_configuration() {
        let mut registry: ReleaseRegistry = ReleaseRegistry::new();

        // Add initial release.
        registry.set_release(
            Machine::Microvm,
            Deployment::SingleProcess,
            128,
            "https://test.com/old.tar.bz2".to_string(),
            "abc111def".to_string(),
        );

        assert_eq!(registry.len(), 1);

        // Update with new release.
        registry.set_release(
            Machine::Microvm,
            Deployment::SingleProcess,
            128,
            "https://test.com/new.tar.bz2".to_string(),
            "abc222def".to_string(),
        );

        // Should still have one entry, but updated.
        assert_eq!(registry.len(), 1);

        let entry: &ReleaseEntry = registry
            .get_release(Machine::Microvm, Deployment::SingleProcess, 128)
            .expect("failed");
        assert_eq!(entry.url(), "https://test.com/new.tar.bz2");
        assert_eq!(entry.commit_id(), "abc222def");
    }

    ///
    /// # Description
    ///
    /// Tests registry serialization to JSON.
    ///
    #[test]
    fn test_serialization() {
        let mut registry: ReleaseRegistry = ReleaseRegistry::new();
        registry.set_release(
            Machine::Microvm,
            Deployment::SingleProcess,
            128,
            "https://example.com/release.tar.bz2".to_string(),
            "abc123def456".to_string(),
        );

        let json: String = serde_json::to_string(&registry).expect("failed");
        assert!(json.contains("releases"));
        assert!(json.contains("microvm-single-process-128mb"));
        assert!(json.contains("https://example.com/release.tar.bz2"));
        assert!(json.contains("abc123def456"));
    }

    ///
    /// # Description
    ///
    /// Tests registry deserialization from JSON.
    ///
    #[test]
    fn test_deserialization() {
        let json: &str = r#"{"releases":{"microvm-single-process-128mb":{"url":"https://example.com/release.tar.bz2","commit_id":"abc123def456"}}}"#;
        let registry: ReleaseRegistry = serde_json::from_str(json).expect("failed");

        let entry: &ReleaseEntry = registry
            .get_release(Machine::Microvm, Deployment::SingleProcess, 128)
            .expect("failed");
        assert_eq!(entry.url(), "https://example.com/release.tar.bz2");
        assert_eq!(entry.commit_id(), "abc123def456");
    }

    ///
    /// # Description
    ///
    /// Tests registry save and load roundtrip.
    ///
    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-registry-save-load");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
        let _: Result<(), std::io::Error> = fs::create_dir_all(&temp_dir).await;

        let mut original: ReleaseRegistry = ReleaseRegistry::new();
        original.set_release(
            Machine::Microvm,
            Deployment::SingleProcess,
            128,
            "https://test.com/file1.tar.bz2".to_string(),
            "abc111def".to_string(),
        );
        original.set_release(
            Machine::Hyperlight,
            Deployment::MultiProcess,
            128,
            "https://test.com/file2.tar.bz2".to_string(),
            "abc222def".to_string(),
        );

        // Save registry.
        let save_result: Result<()> = original.save(&temp_dir).await;
        assert!(save_result.is_ok());

        // Load registry.
        let load_result: Result<ReleaseRegistry> = ReleaseRegistry::load(&temp_dir).await;
        assert!(load_result.is_ok());

        let loaded: ReleaseRegistry = load_result.expect("failed");
        assert_eq!(loaded.len(), 2);

        let entry: &ReleaseEntry = loaded
            .get_release(Machine::Microvm, Deployment::SingleProcess, 128)
            .expect("failed");
        assert_eq!(entry.commit_id(), "abc111def");

        let entry: &ReleaseEntry = loaded
            .get_release(Machine::Hyperlight, Deployment::MultiProcess, 128)
            .expect("failed");
        assert_eq!(entry.commit_id(), "abc222def");

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests that exists returns false when registry doesn't exist.
    ///
    #[tokio::test]
    async fn test_exists_false() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-registry-nonexistent");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;

        let exists: bool = ReleaseRegistry::exists(&temp_dir).await;
        assert!(!exists);
    }

    ///
    /// # Description
    ///
    /// Tests that exists returns true when registry exists.
    ///
    #[tokio::test]
    async fn test_exists_true() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-registry-exists");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
        let _: Result<(), std::io::Error> = fs::create_dir_all(&temp_dir).await;

        let mut registry: ReleaseRegistry = ReleaseRegistry::new();
        registry.set_release(
            Machine::Microvm,
            Deployment::SingleProcess,
            128,
            "https://test.com/file.tar.bz2".to_string(),
            "abc123def456".to_string(),
        );
        let _: Result<()> = registry.save(&temp_dir).await;

        let exists: bool = ReleaseRegistry::exists(&temp_dir).await;
        assert!(exists);

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests registry deletion.
    ///
    #[tokio::test]
    async fn test_delete() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-registry-delete");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
        let _: Result<(), std::io::Error> = fs::create_dir_all(&temp_dir).await;

        let mut registry: ReleaseRegistry = ReleaseRegistry::new();
        registry.set_release(
            Machine::Microvm,
            Deployment::SingleProcess,
            128,
            "https://test.com/file.tar.bz2".to_string(),
            "abc123def456".to_string(),
        );
        let _: Result<()> = registry.save(&temp_dir).await;

        assert!(ReleaseRegistry::exists(&temp_dir).await);

        let delete_result: Result<()> = ReleaseRegistry::delete(&temp_dir).await;
        assert!(delete_result.is_ok());

        assert!(!ReleaseRegistry::exists(&temp_dir).await);

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests loading non-existent registry returns error.
    ///
    #[tokio::test]
    async fn test_load_nonexistent() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-registry-load-nonexistent");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;

        let result: Result<ReleaseRegistry> = ReleaseRegistry::load(&temp_dir).await;
        assert!(result.is_err());
    }

    ///
    /// # Description
    ///
    /// Tests package entry creation.
    ///
    #[test]
    fn test_package_entry_new() {
        let url: String = "https://github.com/nanvix/openssl/release.tar.bz2".to_string();
        let nanvix_commit_id: String = "abc123def456".to_string();
        let entry: PackageEntry = PackageEntry::new(url.clone(), nanvix_commit_id.clone());
        assert_eq!(entry.url(), url);
        assert_eq!(entry.nanvix_commit_id(), nanvix_commit_id);
    }

    ///
    /// # Description
    ///
    /// Tests setting and getting package entries.
    ///
    #[test]
    fn test_set_and_get_package() {
        let mut registry: ReleaseRegistry = ReleaseRegistry::new();

        let url: String = "https://test.com/openssl.tar.bz2".to_string();
        let nanvix_commit_id: String = "abc123def456".to_string();

        // Set a package.
        registry.set_package(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::OpenSSL,
            url.clone(),
            nanvix_commit_id.clone(),
        );

        // Get the package.
        let entry: Option<&PackageEntry> =
            registry.get_package(Machine::Microvm, Deployment::SingleProcess, Package::OpenSSL);
        assert!(entry.is_some());

        let entry: &PackageEntry = entry.expect("package entry should exist");
        assert_eq!(entry.url(), url);
        assert_eq!(entry.nanvix_commit_id(), nanvix_commit_id);
    }

    ///
    /// # Description
    ///
    /// Tests is_package_installed returns true when package is installed with matching commit.
    ///
    #[test]
    fn test_is_package_installed_true() {
        let mut registry: ReleaseRegistry = ReleaseRegistry::new();
        let nanvix_commit_id: String = "abc123def456".to_string();

        registry.set_package(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::OpenSSL,
            "https://test.com/openssl.tar.bz2".to_string(),
            nanvix_commit_id.clone(),
        );

        assert!(registry.is_package_installed(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::OpenSSL,
            &nanvix_commit_id
        ));
    }

    ///
    /// # Description
    ///
    /// Tests is_package_installed returns false when package is not installed.
    ///
    #[test]
    fn test_is_package_installed_false_not_installed() {
        let registry: ReleaseRegistry = ReleaseRegistry::new();

        assert!(!registry.is_package_installed(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::OpenSSL,
            "abc123def456"
        ));
    }

    ///
    /// # Description
    ///
    /// Tests is_package_installed returns false when commit ID does not match.
    ///
    #[test]
    fn test_is_package_installed_false_different_commit() {
        let mut registry: ReleaseRegistry = ReleaseRegistry::new();

        registry.set_package(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::OpenSSL,
            "https://test.com/openssl.tar.bz2".to_string(),
            "abc123def456".to_string(),
        );

        // Check with different commit ID.
        assert!(!registry.is_package_installed(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::OpenSSL,
            "different_commit"
        ));
    }

    ///
    /// # Description
    ///
    /// Tests multiple packages in registry.
    ///
    #[test]
    fn test_multiple_packages() {
        let mut registry: ReleaseRegistry = ReleaseRegistry::new();
        let nanvix_commit_id: String = "abc123def456".to_string();

        // Add multiple packages.
        registry.set_package(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::OpenSSL,
            "https://test.com/openssl.tar.bz2".to_string(),
            nanvix_commit_id.clone(),
        );

        registry.set_package(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::Zlib,
            "https://test.com/zlib.tar.bz2".to_string(),
            nanvix_commit_id.clone(),
        );

        registry.set_package(
            Machine::Hyperlight,
            Deployment::MultiProcess,
            Package::CPython,
            "https://test.com/cpython.tar.bz2".to_string(),
            nanvix_commit_id.clone(),
        );

        // Verify all packages are installed.
        assert!(registry.is_package_installed(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::OpenSSL,
            &nanvix_commit_id
        ));

        assert!(registry.is_package_installed(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::Zlib,
            &nanvix_commit_id
        ));

        assert!(registry.is_package_installed(
            Machine::Hyperlight,
            Deployment::MultiProcess,
            Package::CPython,
            &nanvix_commit_id
        ));

        // Verify non-installed package returns false.
        assert!(!registry.is_package_installed(
            Machine::Microvm,
            Deployment::SingleProcess,
            Package::QuickJS,
            &nanvix_commit_id
        ));
    }
}
