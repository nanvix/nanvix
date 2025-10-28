// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::path::{
    Path,
    PathBuf,
};
use ::syslog::{
    debug,
    error,
    info,
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
/// Metadata about a cached release.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReleaseMetadata {
    /// URL of the release tarball.
    pub(crate) url: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ReleaseMetadata {
    ///
    /// # Description
    ///
    /// Creates a new release metadata instance.
    ///
    /// # Parameters
    ///
    /// - `url`: The URL of the release tarball.
    ///
    /// # Returns
    ///
    /// A new `ReleaseMetadata` instance.
    ///
    pub(crate) fn new(url: String) -> Self {
        Self { url }
    }

    ///
    /// # Description
    ///
    /// Saves the release metadata to a file in the specified directory.
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
                let reason: String = format!("Failed to serialize metadata: {}", error);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        if let Err(error) = fs::write(&metadata_path, json).await {
            let reason: String = format!("Failed to write metadata file: {}", error);
            error!("{reason}");
            anyhow::bail!(reason)
        }

        debug!("Saved release metadata to: {:?}", metadata_path);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Loads release metadata from a file in the specified directory.
    ///
    /// # Parameters
    ///
    /// - `cache_dir`: The directory where the metadata file is located.
    ///
    /// # Returns
    ///
    /// On success, this function returns the loaded `ReleaseMetadata`. On failure, it returns an
    /// object that describes the error.
    ///
    pub(crate) async fn load(cache_dir: &Path) -> Result<Self> {
        let metadata_path: PathBuf = cache_dir.join(METADATA_FILE_NAME);

        // Check if metadata file exists.
        if fs::metadata(&metadata_path).await.is_err() {
            let reason: String = "Metadata file not found".to_string();
            debug!("{reason}");
            anyhow::bail!(reason)
        }

        let json: String = match fs::read_to_string(&metadata_path).await {
            Ok(content) => content,
            Err(error) => {
                let reason: String = format!("Failed to read metadata file: {}", error);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        let metadata: ReleaseMetadata = match serde_json::from_str(&json) {
            Ok(metadata) => metadata,
            Err(error) => {
                let reason: String = format!("Failed to deserialize metadata: {}", error);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        debug!("Loaded release metadata from: {:?}", metadata_path);
        Ok(metadata)
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
                let reason: String = format!("Failed to delete metadata file: {}", error);
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
mod tests {
    use super::*;
    use ::std::env;

    ///
    /// # Description
    ///
    /// Tests metadata creation.
    ///
    #[test]
    fn test_new() {
        let url: String = "https://github.com/test/release.tar.bz2".to_string();
        let metadata: ReleaseMetadata = ReleaseMetadata::new(url.clone());
        assert_eq!(metadata.url, url);
    }

    ///
    /// # Description
    ///
    /// Tests metadata serialization to JSON.
    ///
    #[test]
    fn test_serialization() {
        let metadata: ReleaseMetadata = ReleaseMetadata {
            url: "https://example.com/release.tar.bz2".to_string(),
        };

        let json: String = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("url"));
        assert!(json.contains("https://example.com/release.tar.bz2"));
    }

    ///
    /// # Description
    ///
    /// Tests metadata deserialization from JSON.
    ///
    #[test]
    fn test_deserialization() {
        let json: &str = r#"{"url":"https://example.com/release.tar.bz2"}"#;
        let metadata: ReleaseMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(metadata.url, "https://example.com/release.tar.bz2");
    }

    ///
    /// # Description
    ///
    /// Tests metadata save and load roundtrip.
    ///
    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-metadata-save-load");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
        let _: Result<(), std::io::Error> = fs::create_dir_all(&temp_dir).await;

        let original: ReleaseMetadata =
            ReleaseMetadata::new("https://test.com/file.tar.bz2".to_string());

        // Save metadata.
        let save_result: Result<()> = original.save(&temp_dir).await;
        assert!(save_result.is_ok());

        // Load metadata.
        let load_result: Result<ReleaseMetadata> = ReleaseMetadata::load(&temp_dir).await;
        assert!(load_result.is_ok());

        let loaded: ReleaseMetadata = load_result.unwrap();
        assert_eq!(original.url, loaded.url);

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests that exists returns false when metadata doesn't exist.
    ///
    #[tokio::test]
    async fn test_exists_false() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-metadata-nonexistent");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;

        let exists: bool = ReleaseMetadata::exists(&temp_dir).await;
        assert!(!exists);
    }

    ///
    /// # Description
    ///
    /// Tests that exists returns true when metadata exists.
    ///
    #[tokio::test]
    async fn test_exists_true() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-metadata-exists");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
        let _: Result<(), std::io::Error> = fs::create_dir_all(&temp_dir).await;

        let metadata: ReleaseMetadata =
            ReleaseMetadata::new("https://test.com/file.tar.bz2".to_string());
        let _: Result<()> = metadata.save(&temp_dir).await;

        let exists: bool = ReleaseMetadata::exists(&temp_dir).await;
        assert!(exists);

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests metadata deletion.
    ///
    #[tokio::test]
    async fn test_delete() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-metadata-delete");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
        let _: Result<(), std::io::Error> = fs::create_dir_all(&temp_dir).await;

        let metadata: ReleaseMetadata =
            ReleaseMetadata::new("https://test.com/file.tar.bz2".to_string());
        let _: Result<()> = metadata.save(&temp_dir).await;

        assert!(ReleaseMetadata::exists(&temp_dir).await);

        let delete_result: Result<()> = ReleaseMetadata::delete(&temp_dir).await;
        assert!(delete_result.is_ok());

        assert!(!ReleaseMetadata::exists(&temp_dir).await);

        // Cleanup.
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests loading non-existent metadata returns error.
    ///
    #[tokio::test]
    async fn test_load_nonexistent() {
        let temp_dir: PathBuf = env::temp_dir().join("nanvix-test-metadata-load-nonexistent");
        let _: Result<(), std::io::Error> = fs::remove_dir_all(&temp_dir).await;

        let result: Result<ReleaseMetadata> = ReleaseMetadata::load(&temp_dir).await;
        assert!(result.is_err());
    }
}
