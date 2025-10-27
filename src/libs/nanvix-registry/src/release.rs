// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    deployment::Deployment,
    machine::Machine,
    tarball::Tarball,
    tempfile::TemporaryFile,
};
use ::anyhow::Result;
use ::reqwest::{
    Client,
    Response,
};
use ::std::{
    env,
    path::PathBuf,
};
use ::syslog::{
    error,
    info,
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
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LatestRelease {
    ///
    /// # Description
    ///
    /// Creates a new handle for a latest release for the specified deployment and machine type.
    ///
    /// # Parameters
    ///
    /// - `deployment`: The deployment type.
    /// - `machine`: The target machine type.
    ///
    /// # Returns
    ///
    /// A new handle for a latest release for the specified deployment and machine type.
    ///
    pub(crate) fn new(deployment: Deployment, machine: Machine) -> Self {
        Self {
            deployment,
            machine,
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
    pub(crate) async fn download(&self, dir: &PathBuf) -> Result<String> {
        let release_url: String = self.get_url().await?;

        info!("Downloading release from: {}", release_url);

        // Download the tarball.
        let response: Response = match reqwest::get(&release_url).await {
            Ok(response) => response,
            Err(error) => {
                let reason: String = format!("Failed to download release: {}", error);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => {
                let reason: String = format!("Failed to read release: {}", error);
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
        let client: Client = Client::new();
        let response: Response = match client
            .get(GITHUB_API_URL)
            .header("User-Agent", "nanvix-embedded")
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let reason: String = format!("Failed to fetch releases: {}", error);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        let response: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(error) => {
                let reason: String = format!("Failed to parse releases: {}", error);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Find the release asset URL.
        let assets: &Vec<serde_json::Value> = match response["assets"].as_array() {
            Some(assets) => assets,
            None => {
                let reason: String = "No assets found in release".to_string();
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        let release_pattern: String =
            format!("nanvix-{}-{}-release", self.machine, self.deployment);

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
        let release: LatestRelease = LatestRelease::new(deployment, machine);

        assert!(matches!(release.deployment, Deployment::SingleProcess));
        assert!(matches!(release.machine, Machine::Microvm));
    }

    ///
    /// # Description
    ///
    /// Tests release pattern construction.
    ///
    #[test]
    fn test_release_pattern() {
        let deployment: Deployment = Deployment::MultiProcess;
        let machine: Machine = Machine::Hyperlight;

        let pattern: String = format!("nanvix-{}-{}-release", machine, deployment);
        assert_eq!(pattern, "nanvix-hyperlight-multi-process-release");
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

        for deployment in &deployments {
            for machine in &machines {
                let pattern: String = format!("nanvix-{}-{}-release", machine, deployment);
                assert!(pattern.contains("nanvix-"));
                assert!(pattern.contains("-release"));
            }
        }
    }
}
