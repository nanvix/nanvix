// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    deployment::Deployment,
    github_client,
    machine::Machine,
    rate_limiter::{
        self,
        RateLimiter,
    },
    tarball::Tarball,
    tempfile::TemporaryFile,
};
use ::anyhow::Result;
use ::log::{
    error,
    info,
};
use ::reqwest::{
    Client,
    Response,
};
use ::std::{
    env,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// GitHub API URL for fetching the latest release.
const GITHUB_API_URL: &str = "https://api.github.com/repos/nanvix/nanvix/releases/latest";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents the latest release from the Nanvix GitHub repository.
///
pub(crate) struct LatestRelease {
    /// Deployment type for the release.
    deployment: Deployment,
    /// Target machine type for the release.
    machine: Machine,
    /// Memory size in megabytes for selecting the correct release archive.
    memory_size_mb: u32,
    /// Rate limiter for GitHub API calls.
    rate_limiter: RateLimiter,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LatestRelease {
    ///
    /// # Description
    ///
    /// Creates a new handle for a latest release for the specified deployment, machine type, and
    /// memory size.
    ///
    /// # Parameters
    ///
    /// - `deployment`: The deployment type.
    /// - `machine`: The target machine type.
    /// - `memory_size_mb`: The memory size in megabytes.
    ///
    /// # Returns
    ///
    /// A new handle for a latest release for the specified deployment, machine type, and memory
    /// size.
    ///
    pub(crate) fn new(deployment: Deployment, machine: Machine, memory_size_mb: u32) -> Self {
        Self {
            deployment,
            machine,
            memory_size_mb,
            rate_limiter: RateLimiter::for_github(),
        }
    }

    ///
    /// # Description
    ///
    /// Downloads the latest release tarball from GitHub and extracts it to the specified directory.
    ///
    /// # Parameters
    ///
    /// - `dir`: The directory where the release will be extracted.
    ///
    /// # Returns
    ///
    /// On success, this function returns the URL of the downloaded release. On failure, it returns
    /// an object that describes the error.
    ///
    pub(crate) async fn download(&self, dir: &Path) -> Result<String> {
        let release_url: String = self.get_url().await?;

        info!("Downloading release from: {}", release_url);

        // Apply rate limiting before download.
        self.rate_limiter.wait().await;

        // Download the tarball.
        let client: Client = github_client::build_client()?;
        let response: Response = match github_client::authenticated_get(&client, &release_url)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let reason: String = format!("Failed to download release: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Check for rate limit errors.
        rate_limiter::check_rate_limit(&response)?;

        let bytes: ::bytes::Bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => {
                let reason: String = format!("Failed to read release: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Save to temp file.
        let temp_path: PathBuf =
            env::temp_dir().join(format!("nanvix-release-{}.tar.bz2", uuid::Uuid::new_v4()));
        let temp_file: TemporaryFile = TemporaryFile::new(temp_path);
        temp_file.write(&bytes).await?;

        // Extract tarball.
        info!("Extracting release...");
        let tarball: Tarball = Tarball::open(temp_file.path())?;
        tarball.extract(dir).await?;

        Ok(release_url)
    }

    ///
    /// # Description
    ///
    /// Fetches the download URL for the latest release from the GitHub API.
    ///
    /// # Returns
    ///
    /// On success, this function returns the download URL as a string. On failure, it returns an
    /// object that describes the error.
    ///
    pub(crate) async fn get_url(&self) -> Result<String> {
        // Apply rate limiting before API call.
        self.rate_limiter.wait().await;

        let client: Client = github_client::build_client()?;
        let response: Response = match github_client::authenticated_get(&client, GITHUB_API_URL)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let reason: String = format!("Failed to fetch releases: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Check for rate limit errors.
        rate_limiter::check_rate_limit(&response)?;

        let response: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(error) => {
                let reason: String = format!("Failed to parse releases: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Find the release asset URL.
        let assets: &[serde_json::Value] = match response["assets"].as_array() {
            Some(assets) => assets,
            None => {
                let reason: String = "No assets found in release".to_string();
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        let release_pattern: String = format!(
            "nanvix-{}-{}-release-{}mb-",
            self.machine, self.deployment, self.memory_size_mb
        );

        // Search for the matching asset.
        for asset in assets {
            if let Some(name) = asset["name"].as_str() {
                if name.contains(&release_pattern) && Tarball::is_supported(name) {
                    if let Some(url) = asset["browser_download_url"].as_str() {
                        return Ok(url.to_string());
                    }
                }
            }
        }

        let reason: String = "Could not find release tarball in latest release".to_string();
        error!("{reason}");
        anyhow::bail!(reason)
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
    /// Tests LatestRelease creation.
    ///
    #[test]
    fn test_new() {
        let deployment: Deployment = Deployment::SingleProcess;
        let machine: Machine = Machine::Microvm;
        let release: LatestRelease = LatestRelease::new(deployment, machine, 128);

        assert!(matches!(release.deployment, Deployment::SingleProcess));
        assert!(matches!(release.machine, Machine::Microvm));
        assert_eq!(release.memory_size_mb, 128);
    }

    ///
    /// # Description
    ///
    /// Tests release pattern construction with memory size.
    ///
    #[test]
    fn test_release_pattern() {
        let deployment: Deployment = Deployment::MultiProcess;
        let machine: Machine = Machine::Hyperlight;

        let pattern: String = format!("nanvix-{}-{}-release-{}mb-", machine, deployment, 128);
        assert_eq!(pattern, "nanvix-hyperlight-multi-process-release-128mb-");

        // Verify the pattern matches the memory-size archive name format.
        let name_128: &str = "nanvix-hyperlight-multi-process-release-128mb-abc123def456.tar.bz2";
        assert!(name_128.contains(&pattern));

        // Verify the pattern does NOT match a different memory size.
        let name_1024: &str = "nanvix-hyperlight-multi-process-release-1024mb-abc123def456.tar.bz2";
        assert!(!name_1024.contains(&pattern));
    }

    ///
    /// # Description
    ///
    /// Tests GitHub API URL constant.
    ///
    #[test]
    fn test_github_api_url() {
        assert_eq!(GITHUB_API_URL, "https://api.github.com/repos/nanvix/nanvix/releases/latest");
        assert!(GITHUB_API_URL.starts_with("https://"));
        assert!(GITHUB_API_URL.contains("github.com"));
    }

    ///
    /// # Description
    ///
    /// Tests release pattern for all combinations.
    ///
    #[test]
    fn test_all_release_patterns() {
        let deployments: [Deployment; 2] = [Deployment::SingleProcess, Deployment::MultiProcess];
        let machines: [Machine; 2] = [Machine::Hyperlight, Machine::Microvm];
        let memory_sizes: [u32; 4] = [128, 256, 512, 1024];

        for deployment in &deployments {
            for machine in &machines {
                for memory_size_mb in &memory_sizes {
                    let pattern: String =
                        format!("nanvix-{}-{}-release-{}mb-", machine, deployment, memory_size_mb);
                    assert!(pattern.contains("nanvix-"));
                    assert!(pattern.contains("-release-"));
                    assert!(pattern.ends_with("mb-"));
                }
            }
        }
    }
}
