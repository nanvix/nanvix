// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    checksum::verify_sha256,
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
use ::bytes::Bytes;
use ::log::{
    debug,
    error,
    info,
    warn,
};
use ::reqwest::{
    Client,
    Response,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::{
    env,
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
};
use ::tokio::sync::OnceCell;

//==================================================================================================
// Constants
//==================================================================================================

/// GitHub owner for Nanvix packages.
const GITHUB_OWNER: &str = "nanvix";

/// Name of the dependencies metadata file within package releases.
const DEPENDENCIES_FILE_NAME: &str = "dependencies.json";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents a Nanvix package that can be installed from GitHub releases.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Package {
    /// OpenBLAS library.
    OpenBLAS,
    /// OpenSSL library.
    OpenSSL,
    /// SQLite library.
    SQLite,
    /// Zlib compression library.
    Zlib,
    /// QuickJS JavaScript engine.
    QuickJS,
    /// CPython interpreter.
    CPython,
}

///
/// # Description
///
/// Metadata about package dependencies.
///
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct PackageDependencies {
    /// List of package dependencies.
    dependencies: Vec<String>,
}

///
/// # Description
///
/// Represents a package release from a Nanvix package repository.
///
pub(crate) struct PackageRelease {
    /// The package to download.
    package: Package,
    /// Deployment type for the release.
    deployment: Deployment,
    /// Target machine type for the release.
    machine: Machine,
    /// Target Nanvix commit ID.
    nanvix_commit_id: String,
    /// Cached release info to avoid duplicate API calls.
    cached_release_info: Arc<OnceCell<serde_json::Value>>,
    /// Whether to fall back to latest release if compatible version not found.
    use_latest_fallback: bool,
    /// Rate limiter for GitHub API calls.
    rate_limiter: RateLimiter,
    /// Reusable HTTP client.
    client: Client,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Package {
    /// String representation of OpenBLAS package.
    pub const OPENBLAS_STR: &'static str = "openblas";
    /// String representation of OpenSSL package.
    pub const OPENSSL_STR: &'static str = "openssl";
    /// String representation of SQLite package.
    pub const SQLITE_STR: &'static str = "sqlite";
    /// String representation of Zlib package.
    pub const ZLIB_STR: &'static str = "zlib";
    /// String representation of QuickJS package.
    pub const QUICKJS_STR: &'static str = "quickjs";
    /// String representation of CPython package.
    pub const CPYTHON_STR: &'static str = "cpython";

    ///
    /// # Description
    ///
    /// Converts the package to its string representation.
    ///
    /// # Returns
    ///
    /// A string representation of the package.
    ///
    pub fn as_str(&self) -> &'static str {
        match self {
            Package::OpenBLAS => Package::OPENBLAS_STR,
            Package::OpenSSL => Package::OPENSSL_STR,
            Package::SQLite => Package::SQLITE_STR,
            Package::Zlib => Package::ZLIB_STR,
            Package::QuickJS => Package::QUICKJS_STR,
            Package::CPython => Package::CPYTHON_STR,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the GitHub repository name for this package.
    ///
    /// # Returns
    ///
    /// The repository name as a string.
    ///
    pub fn repo_name(&self) -> &'static str {
        match self {
            Package::OpenBLAS => "OpenBLAS",
            Package::OpenSSL => "openssl",
            Package::SQLite => "sqlite",
            Package::Zlib => "zlib",
            Package::QuickJS => "quickjs",
            Package::CPython => "cpython",
        }
    }

    ///
    /// # Description
    ///
    /// Returns the GitHub API URL for fetching the latest release of this package.
    ///
    /// # Returns
    ///
    /// The GitHub API URL as a string.
    ///
    pub fn api_url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            GITHUB_OWNER,
            self.repo_name()
        )
    }

    ///
    /// # Description
    ///
    /// Returns the GitHub API URL for fetching all releases of this package.
    ///
    /// # Returns
    ///
    /// The GitHub API URL as a string.
    ///
    pub fn api_url_all_releases(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=100",
            GITHUB_OWNER,
            self.repo_name()
        )
    }

    ///
    /// # Description
    ///
    /// Returns a list of all available packages.
    ///
    /// # Returns
    ///
    /// A vector containing all package variants.
    ///
    pub fn all() -> Vec<Package> {
        vec![
            Package::OpenBLAS,
            Package::OpenSSL,
            Package::SQLite,
            Package::Zlib,
            Package::QuickJS,
            Package::CPython,
        ]
    }
}

impl ::std::fmt::Display for Package {
    ///
    /// # Description
    ///
    /// Converts the package to its string representation.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter.
    ///
    /// # Returns
    ///
    /// On success, this function returns an empty tuple. On failure, it returns an object that
    /// describes the error.
    ///
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for Package {
    type Error = anyhow::Error;

    ///
    /// # Description
    ///
    /// Attempts to convert a string slice to a `Package` enum variant.
    ///
    /// # Parameters
    ///
    /// - `value`: The string representation of the package (case-insensitive).
    ///
    /// # Returns
    ///
    /// On success, returns the corresponding `Package` variant. On failure, it returns an object
    /// that describes the error.
    ///
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value_lower: String = value.to_lowercase();
        match value_lower.as_str() {
            Self::OPENBLAS_STR => Ok(Package::OpenBLAS),
            Self::OPENSSL_STR => Ok(Package::OpenSSL),
            Self::SQLITE_STR => Ok(Package::SQLite),
            Self::ZLIB_STR => Ok(Package::Zlib),
            Self::QUICKJS_STR => Ok(Package::QuickJS),
            Self::CPYTHON_STR | "python" | "python3" => Ok(Package::CPython),
            _ => {
                let reason: String = format!("Unknown package: {value}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        }
    }
}

impl PackageDependencies {
    ///
    /// # Description
    ///
    /// Creates a new empty package dependencies instance.
    ///
    /// # Returns
    ///
    /// A new `PackageDependencies` instance with no dependencies.
    ///
    pub(crate) fn new() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the list of package dependencies.
    ///
    /// # Returns
    ///
    /// A slice of dependency package names.
    ///
    pub(crate) fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    ///
    /// # Description
    ///
    /// Parses package dependencies from a JSON string.
    ///
    /// # Parameters
    ///
    /// - `json`: The JSON string containing dependency information.
    ///
    /// # Returns
    ///
    /// On success, returns a `PackageDependencies` instance. On failure, returns an error.
    ///
    pub(crate) fn from_json(json: &str) -> Result<Self> {
        match serde_json::from_str(json) {
            Ok(deps) => Ok(deps),
            Err(error) => {
                let reason: String = format!("Failed to parse dependencies: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        }
    }
}

impl PackageRelease {
    ///
    /// # Description
    ///
    /// Creates a new handle for a package release.
    ///
    /// # Parameters
    ///
    /// - `package`: The package to download.
    /// - `deployment`: The deployment type.
    /// - `machine`: The target machine type.
    /// - `nanvix_commit_id`: The Nanvix commit ID to match.
    /// - `use_latest_fallback`: If true, falls back to latest release when compatible version
    ///   is not found. If false, returns an error.
    ///
    /// # Returns
    ///
    /// On success, a new handle for the package release. On failure, returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    ///
    pub(crate) fn new(
        package: Package,
        deployment: Deployment,
        machine: Machine,
        nanvix_commit_id: String,
        use_latest_fallback: bool,
    ) -> Result<Self> {
        let client: Client = github_client::build_client()?;
        Ok(Self {
            package,
            deployment,
            machine,
            nanvix_commit_id,
            cached_release_info: Arc::new(OnceCell::new()),
            use_latest_fallback,
            rate_limiter: RateLimiter::for_github(),
            client,
        })
    }

    /// Returns the package type.
    #[cfg(test)]
    fn package(&self) -> Package {
        self.package
    }

    /// Returns the deployment type.
    #[cfg(test)]
    fn deployment(&self) -> Deployment {
        self.deployment
    }

    /// Returns the target machine type.
    #[cfg(test)]
    fn machine(&self) -> Machine {
        self.machine
    }

    /// Returns the Nanvix commit ID.
    #[cfg(test)]
    fn nanvix_commit_id(&self) -> &str {
        &self.nanvix_commit_id
    }

    /// Returns whether the latest fallback is enabled.
    #[cfg(test)]
    fn use_latest_fallback(&self) -> bool {
        self.use_latest_fallback
    }

    ///
    /// # Description
    ///
    /// Downloads the package release tarball from GitHub and extracts it to the specified
    /// directory.
    ///
    /// # Parameters
    ///
    /// - `dir`: The directory where the release will be extracted.
    ///
    /// # Returns
    ///
    /// On success, returns the URL of the downloaded release. On failure, returns an error.
    ///
    pub(crate) async fn download(&self, dir: &Path) -> Result<String> {
        let release_url: String = self.get_url().await?;

        info!("Downloading package {} from: {}", self.package, release_url);

        // Apply rate limiting before download.
        self.rate_limiter.wait().await;

        // Download the tarball.
        let response: Response = match github_client::authenticated_get(&self.client, &release_url)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let reason: String =
                    format!("Failed to download package {}: {error}", self.package);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Check for rate limit errors.
        rate_limiter::check_rate_limit(&response)?;

        let bytes: Bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => {
                let reason: String = format!("Failed to read package {}: {error}", self.package);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Verify checksum if available.
        if let Some(expected_checksum) = self.get_checksum().await? {
            debug!("Verifying checksum for package {}", self.package);
            if !verify_sha256(&bytes, &expected_checksum) {
                let reason: String =
                    format!("Checksum verification failed for package {}", self.package);
                error!("{reason}");
                anyhow::bail!(reason)
            }
            info!("Checksum verified for package {}", self.package);
        } else {
            debug!("No checksum available for package {}, skipping verification", self.package);
        }

        // Save to temp file.
        let temp_path: PathBuf = env::temp_dir().join(format!(
            "nanvix-package-{}-{}.tar.bz2",
            self.package,
            uuid::Uuid::new_v4()
        ));
        let temp_file: TemporaryFile = TemporaryFile::new(temp_path);
        temp_file.write(&bytes).await?;

        // Extract tarball.
        info!("Extracting package {}...", self.package);
        let tarball: Tarball = Tarball::open(temp_file.path())?;
        tarball.extract(dir).await?;

        Ok(release_url)
    }

    ///
    /// # Description
    ///
    /// Fetches the release information from GitHub API.
    ///
    /// This is a helper method that fetches and validates the release metadata, caching
    /// the result to avoid duplicate API calls between `get_url()` and `get_dependencies()`.
    ///
    /// First attempts to find a release matching the target Nanvix commit ID. If no compatible
    /// release is found and `use_latest_fallback` is true, falls back to the latest available
    /// release with a warning. Otherwise, returns an error.
    ///
    /// # Returns
    ///
    /// On success, returns the parsed JSON release information. On failure, returns an error.
    ///
    async fn fetch_release_info(&self) -> Result<serde_json::Value> {
        // Use cached release info if available.
        if let Some(cached) = self.cached_release_info.get() {
            debug!("Using cached release info for package {}", self.package);
            return Ok(cached.clone());
        }

        // Extract the short commit ID (first 7 characters) for comparison.
        // TODO: use full commit ID for matching (https://github.com/nanvix/nanvix/issues/1358)
        let short_commit_id: &str = if self.nanvix_commit_id.len() >= 7 {
            &self.nanvix_commit_id[..7]
        } else {
            &self.nanvix_commit_id
        };

        // First, try to find a release matching the target commit ID by fetching all releases.
        let all_releases_url: String = self.package.api_url_all_releases();

        // Apply rate limiting before API call.
        self.rate_limiter.wait().await;

        let response: Response =
            match github_client::authenticated_get(&self.client, &all_releases_url)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let reason: String =
                        format!("Failed to fetch package {} releases: {error}", self.package);
                    error!("{reason}");
                    anyhow::bail!(reason)
                },
            };

        // Check for rate limit errors.
        rate_limiter::check_rate_limit(&response)?;

        let all_releases: Vec<serde_json::Value> = match response.json().await {
            Ok(json) => json,
            Err(error) => {
                let reason: String =
                    format!("Failed to parse package {} releases: {error}", self.package);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Search for a release matching the target commit ID.
        // The commit ID should appear at a word boundary to avoid false positives
        // (e.g., matching version numbers that coincidentally contain the same digits).
        for release in all_releases.iter() {
            let tag_name: Option<&str> = release["tag_name"].as_str();
            if let Some(tag_name) = tag_name {
                if Self::tag_matches_commit(tag_name, short_commit_id) {
                    debug!(
                        "Found compatible release for package {}: tag '{}'",
                        self.package, tag_name
                    );
                    let _ = self.cached_release_info.set(release.clone());
                    return Ok(release.clone());
                }
            }
        }

        // No compatible release found.
        if !self.use_latest_fallback {
            let reason: String = format!(
                "No compatible release found for package {} with commit '{}'",
                self.package, short_commit_id
            );
            error!("{reason}");
            anyhow::bail!(reason)
        }

        // Fall back to the latest release.
        warn!(
            "No compatible release found for package {} with commit '{}', using latest release.",
            self.package, short_commit_id
        );

        // Fetch the latest release.
        let latest_url: String = self.package.api_url();

        // Apply rate limiting before API call.
        self.rate_limiter.wait().await;

        let response: Response = match github_client::authenticated_get(&self.client, &latest_url)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let reason: String =
                    format!("Failed to fetch latest package {} release: {error}", self.package);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Check for rate limit errors.
        rate_limiter::check_rate_limit(&response)?;

        let release_info: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(error) => {
                let reason: String =
                    format!("Failed to parse latest package {} release: {error}", self.package);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Verify the release has a tag.
        let tag_name: &str = match release_info["tag_name"].as_str() {
            Some(tag) => tag,
            None => {
                let reason: String = format!("No tag found for package {} release", self.package);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        warn!(
            "Using latest release '{}' for package {} (target commit: '{}')",
            tag_name, self.package, short_commit_id
        );

        // Cache the release info for future calls.
        let _ = self.cached_release_info.set(release_info.clone());

        Ok(release_info)
    }

    ///
    /// # Description
    ///
    /// Fetches the download URL for the package release from the GitHub API.
    ///
    /// The release is identified by checking if the release tag contains the nanvix commit ID.
    /// The asset is matched by the pattern: `<package>-<machine>-<deployment>.tar.bz2`
    ///
    /// # Returns
    ///
    /// On success, returns the download URL as a string. On failure, returns an error.
    ///
    pub(crate) async fn get_url(&self) -> Result<String> {
        let release_info: serde_json::Value = self.fetch_release_info().await?;

        // Find the release asset URL.
        let assets: &[serde_json::Value] = match release_info["assets"].as_array() {
            Some(assets) => assets,
            None => {
                let reason: String = format!("No assets found in package {} release", self.package);
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Build the expected asset name: <package>-<machine>-<deployment>.tar.bz2
        let expected_asset_name: String =
            format!("{}-{}-{}.tar.bz2", self.package, self.machine, self.deployment);

        debug!("Searching for package asset: {}", expected_asset_name);

        // Search for the matching asset.
        for asset in assets.iter() {
            let name: Option<&str> = asset["name"].as_str();
            if let Some(name) = name {
                if name == expected_asset_name {
                    if let Some(url) = asset["browser_download_url"].as_str() {
                        debug!("Found matching package asset: {}", name);
                        return Ok(url.to_string());
                    }
                }
            }
        }

        let reason: String = format!(
            "Could not find package {} release for {}-{}",
            self.package, self.machine, self.deployment
        );
        error!("{reason}");
        anyhow::bail!(reason)
    }

    ///
    /// # Description
    ///
    /// Fetches the package dependencies from the dependencies metadata file.
    ///
    /// This method downloads the `dependencies.json` file from the package release and parses it
    /// to extract the list of required dependencies.
    ///
    /// # Returns
    ///
    /// On success, returns a `PackageDependencies` instance. On failure, returns an error.
    /// If no dependencies file exists, returns an empty dependencies instance.
    ///
    pub(crate) async fn get_dependencies(&self) -> Result<PackageDependencies> {
        let release_info: serde_json::Value = self.fetch_release_info().await?;

        // Find the dependencies file asset.
        let assets: &[serde_json::Value] = match release_info["assets"].as_array() {
            Some(assets) => assets,
            None => {
                debug!("No assets in package {} release, no dependencies", self.package);
                return Ok(PackageDependencies::new());
            },
        };

        // Build the pattern to match: dependencies-<machine>-<deployment>.json or dependencies.json
        let deps_pattern: String =
            format!("dependencies-{}-{}.json", self.machine, self.deployment);

        debug!("Searching for dependencies file matching pattern: {}", deps_pattern);

        // Search for the dependencies file. Prefer machine-deployment-specific file over generic.
        let mut generic_asset: Option<&serde_json::Value> = None;
        for asset in assets.iter() {
            let name: Option<&str> = asset["name"].as_str();
            if let Some(name) = name {
                if name == deps_pattern {
                    // Exact match for machine-deployment-specific file, use immediately.
                    return self.download_dependencies(asset).await;
                } else if name == DEPENDENCIES_FILE_NAME {
                    // Generic dependencies file, save as fallback.
                    generic_asset = Some(asset);
                }
            }
        }

        // Fall back to generic dependencies file if found.
        if let Some(asset) = generic_asset {
            return self.download_dependencies(asset).await;
        }

        // No dependencies file found, return empty dependencies.
        debug!("No dependencies file found for package {}", self.package);
        Ok(PackageDependencies::new())
    }

    ///
    /// # Description
    ///
    /// Downloads and parses a dependencies file from a release asset.
    ///
    /// # Parameters
    ///
    /// - `asset`: The JSON object representing the release asset containing the dependencies file.
    ///
    /// # Returns
    ///
    /// On success, returns a `PackageDependencies` instance. On failure, returns an error.
    ///
    async fn download_dependencies(
        &self,
        asset: &serde_json::Value,
    ) -> Result<PackageDependencies> {
        let url: &str = match asset["browser_download_url"].as_str() {
            Some(url) => url,
            None => {
                let reason: String = "Dependencies asset has no download URL".to_string();
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        let name: &str = asset["name"].as_str().unwrap_or("unknown");
        debug!("Found dependencies file: {}", name);

        // Apply rate limiting before download.
        self.rate_limiter.wait().await;

        // Download the dependencies file.
        let deps_response: Response = match github_client::authenticated_get(&self.client, url)
            .send()
            .await
        {
            Ok(r) => r,
            Err(error) => {
                let reason: String = format!("Failed to download dependencies file: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        // Check for rate limit errors.
        rate_limiter::check_rate_limit(&deps_response)?;

        let deps_json: String = match deps_response.text().await {
            Ok(text) => text,
            Err(error) => {
                let reason: String = format!("Failed to read dependencies file: {error}");
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        PackageDependencies::from_json(&deps_json)
    }

    ///
    /// # Description
    ///
    /// Fetches the SHA256 checksum for the package release from the release assets.
    ///
    /// This method looks for a checksum file matching the pattern:
    /// `<package>-<machine>-<deployment>.sha256` or `<package>-<machine>-<deployment>.tar.bz2.sha256`
    ///
    /// # Returns
    ///
    /// On success, returns `Some(String)` containing the checksum if a checksum file is found,
    /// or `None` if no checksum file exists. On failure, returns an error.
    ///
    pub(crate) async fn get_checksum(&self) -> Result<Option<String>> {
        let release_info: serde_json::Value = self.fetch_release_info().await?;

        // Find the checksum file asset.
        let assets: &[serde_json::Value] = match release_info["assets"].as_array() {
            Some(assets) => assets,
            None => {
                debug!("No assets in package {} release, no checksum available", self.package);
                return Ok(None);
            },
        };

        // Build expected checksum file names.
        let checksum_name: String =
            format!("{}-{}-{}.sha256", self.package, self.machine, self.deployment);
        let checksum_name_tarball: String =
            format!("{}-{}-{}.tar.bz2.sha256", self.package, self.machine, self.deployment);

        debug!("Searching for checksum file: {} or {}", checksum_name, checksum_name_tarball);

        // Search for the checksum file.
        for asset in assets.iter() {
            let name: Option<&str> = asset["name"].as_str();
            if let Some(name) = name {
                // Check if this is a checksum file for our package.
                if name == checksum_name || name == checksum_name_tarball {
                    if let Some(url) = asset["browser_download_url"].as_str() {
                        debug!("Found checksum file: {}", name);

                        // Apply rate limiting before download.
                        self.rate_limiter.wait().await;

                        // Download the checksum file.
                        let checksum_response: Response =
                            match github_client::authenticated_get(&self.client, url)
                                .send()
                                .await
                            {
                                Ok(r) => r,
                                Err(error) => {
                                    let reason: String =
                                        format!("Failed to download checksum file: {error}");
                                    error!("{reason}");
                                    anyhow::bail!(reason)
                                },
                            };

                        // Check for rate limit errors.
                        rate_limiter::check_rate_limit(&checksum_response)?;

                        let checksum_text: String = match checksum_response.text().await {
                            Ok(text) => text,
                            Err(error) => {
                                let reason: String =
                                    format!("Failed to read checksum file: {error}");
                                error!("{reason}");
                                anyhow::bail!(reason)
                            },
                        };

                        // Parse the checksum (typically in format: "hash  filename" or just "hash").
                        let checksum: String = checksum_text
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_string();

                        if checksum.is_empty() {
                            warn!("Empty checksum in file: {}", name);
                            return Ok(None);
                        }

                        return Ok(Some(checksum));
                    }
                }
            }
        }

        // No checksum file found.
        debug!("No checksum file found for package {}", self.package);
        Ok(None)
    }

    ///
    /// # Description
    ///
    /// Checks if a release tag matches the expected commit ID pattern.
    ///
    /// The function validates that the commit ID appears as a distinct segment in the tag,
    /// not as part of a version number or other identifier. Valid patterns include:
    /// - Tag ending with the commit ID (e.g., "v1.0.0-abc1234")
    /// - Tag containing the commit ID after a separator (e.g., "release-abc1234-build")
    ///
    /// # Parameters
    ///
    /// - `tag`: The release tag name to check.
    /// - `commit_id`: The commit ID (or short commit ID) to match.
    ///
    /// # Returns
    ///
    /// `true` if the tag contains the commit ID as a distinct segment, `false` otherwise.
    ///
    fn tag_matches_commit(tag: &str, commit_id: &str) -> bool {
        // Check if tag ends with the commit ID.
        if tag.ends_with(commit_id) {
            // Verify it's after a separator (not part of another word).
            let prefix_len: usize = tag.len().saturating_sub(commit_id.len());
            if prefix_len == 0 {
                return true; // Tag is exactly the commit ID.
            }
            let prefix_char: char = tag.chars().nth(prefix_len.saturating_sub(1)).unwrap_or('-');
            if !prefix_char.is_alphanumeric() {
                return true;
            }
        }

        // Check if commit ID appears after a separator and before another separator.
        for separator in ['-', '_', '.', '/'] {
            let pattern: String = format!("{}{}{}", separator, commit_id, separator);
            if tag.contains(&pattern) {
                return true;
            }
            // Also check pattern at the end.
            let end_pattern: String = format!("{}{}", separator, commit_id);
            if tag.ends_with(&end_pattern) {
                return true;
            }
        }

        false
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
    /// Tests Package enum string conversion.
    ///
    #[test]
    fn test_package_as_str() {
        assert_eq!(Package::OpenBLAS.as_str(), "openblas");
        assert_eq!(Package::OpenSSL.as_str(), "openssl");
        assert_eq!(Package::SQLite.as_str(), "sqlite");
        assert_eq!(Package::Zlib.as_str(), "zlib");
        assert_eq!(Package::QuickJS.as_str(), "quickjs");
        assert_eq!(Package::CPython.as_str(), "cpython");
    }

    ///
    /// # Description
    ///
    /// Tests Package repo name.
    ///
    #[test]
    fn test_package_repo_name() {
        assert_eq!(Package::OpenBLAS.repo_name(), "OpenBLAS");
        assert_eq!(Package::OpenSSL.repo_name(), "openssl");
        assert_eq!(Package::SQLite.repo_name(), "sqlite");
        assert_eq!(Package::Zlib.repo_name(), "zlib");
        assert_eq!(Package::QuickJS.repo_name(), "quickjs");
        assert_eq!(Package::CPython.repo_name(), "cpython");
    }

    ///
    /// # Description
    ///
    /// Tests Package API URL generation.
    ///
    #[test]
    fn test_package_api_url() {
        assert_eq!(
            Package::OpenBLAS.api_url(),
            "https://api.github.com/repos/nanvix/OpenBLAS/releases/latest"
        );
        assert_eq!(
            Package::CPython.api_url(),
            "https://api.github.com/repos/nanvix/cpython/releases/latest"
        );
    }

    ///
    /// # Description
    ///
    /// Tests Package display trait.
    ///
    #[test]
    fn test_package_display() {
        assert_eq!(format!("{}", Package::OpenBLAS), "openblas");
        assert_eq!(format!("{}", Package::CPython), "cpython");
    }

    ///
    /// # Description
    ///
    /// Tests Package conversion from string.
    ///
    #[test]
    fn test_package_try_from_str() {
        assert!(matches!(Package::try_from("openblas"), Ok(Package::OpenBLAS)));
        assert!(matches!(Package::try_from("OpenBLAS"), Ok(Package::OpenBLAS)));
        assert!(matches!(Package::try_from("openssl"), Ok(Package::OpenSSL)));
        assert!(matches!(Package::try_from("sqlite"), Ok(Package::SQLite)));
        assert!(matches!(Package::try_from("zlib"), Ok(Package::Zlib)));
        assert!(matches!(Package::try_from("quickjs"), Ok(Package::QuickJS)));
        assert!(matches!(Package::try_from("cpython"), Ok(Package::CPython)));
        assert!(matches!(Package::try_from("python"), Ok(Package::CPython)));
        assert!(matches!(Package::try_from("python3"), Ok(Package::CPython)));
        assert!(Package::try_from("unknown").is_err());
    }

    ///
    /// # Description
    ///
    /// Tests Package::all returns all variants.
    ///
    #[test]
    fn test_package_all() {
        let all: Vec<Package> = Package::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&Package::OpenBLAS));
        assert!(all.contains(&Package::OpenSSL));
        assert!(all.contains(&Package::SQLite));
        assert!(all.contains(&Package::Zlib));
        assert!(all.contains(&Package::QuickJS));
        assert!(all.contains(&Package::CPython));
    }

    ///
    /// # Description
    ///
    /// Tests PackageDependencies creation.
    ///
    #[test]
    fn test_package_dependencies_new() {
        let deps: PackageDependencies = PackageDependencies::new();
        assert!(deps.dependencies().is_empty());
    }

    ///
    /// # Description
    ///
    /// Tests PackageDependencies JSON parsing.
    ///
    #[test]
    fn test_package_dependencies_from_json() {
        let json: &str = r#"{"dependencies": ["openssl", "zlib"]}"#;
        let deps: PackageDependencies =
            PackageDependencies::from_json(json).expect("failed to parse dependencies JSON");
        assert_eq!(deps.dependencies().len(), 2);
        assert!(deps.dependencies().contains(&"openssl".to_string()));
        assert!(deps.dependencies().contains(&"zlib".to_string()));
    }

    ///
    /// # Description
    ///
    /// Tests PackageDependencies JSON parsing with empty dependencies.
    ///
    #[test]
    fn test_package_dependencies_from_json_empty() {
        let json: &str = r#"{"dependencies": []}"#;
        let deps: PackageDependencies =
            PackageDependencies::from_json(json).expect("failed to parse empty dependencies JSON");
        assert!(deps.dependencies().is_empty());
    }

    ///
    /// # Description
    ///
    /// Tests PackageDependencies JSON parsing failure.
    ///
    #[test]
    fn test_package_dependencies_from_json_invalid() {
        let json: &str = "invalid json";
        let result: Result<PackageDependencies> = PackageDependencies::from_json(json);
        assert!(result.is_err());
    }

    ///
    /// # Description
    ///
    /// Tests PackageRelease creation.
    ///
    #[test]
    fn test_package_release_new() {
        let release: PackageRelease = PackageRelease::new(
            Package::OpenBLAS,
            Deployment::SingleProcess,
            Machine::Microvm,
            "abc123".to_string(),
            false,
        )
        .expect("failed to create PackageRelease for OpenBLAS");
        assert_eq!(release.package(), Package::OpenBLAS);
        assert!(matches!(release.deployment(), Deployment::SingleProcess));
        assert!(matches!(release.machine(), Machine::Microvm));
        assert_eq!(release.nanvix_commit_id(), "abc123");
        assert!(!release.use_latest_fallback());

        // Test with fallback enabled.
        let release_fallback: PackageRelease = PackageRelease::new(
            Package::OpenSSL,
            Deployment::MultiProcess,
            Machine::Hyperlight,
            "def456".to_string(),
            true,
        )
        .expect("failed to create PackageRelease for OpenSSL");
        assert!(release_fallback.use_latest_fallback());
    }

    ///
    /// # Description
    ///
    /// Tests tag_matches_commit with valid patterns.
    ///
    #[test]
    fn test_tag_matches_commit_valid() {
        // Tag ending with commit ID after separator.
        assert!(PackageRelease::tag_matches_commit("v1.0.0-abc1234", "abc1234"));
        assert!(PackageRelease::tag_matches_commit("release-abc1234", "abc1234"));
        assert!(PackageRelease::tag_matches_commit("v1.0.0_abc1234", "abc1234"));
        assert!(PackageRelease::tag_matches_commit("v1.0.0.abc1234", "abc1234"));

        // Commit ID in the middle with separators.
        assert!(PackageRelease::tag_matches_commit("v1.0.0-abc1234-build", "abc1234"));
        assert!(PackageRelease::tag_matches_commit("release-abc1234-test", "abc1234"));

        // Tag is exactly the commit ID.
        assert!(PackageRelease::tag_matches_commit("abc1234", "abc1234"));
    }

    ///
    /// # Description
    ///
    /// Tests tag_matches_commit with invalid patterns (false positives prevention).
    ///
    #[test]
    fn test_tag_matches_commit_invalid() {
        // Commit ID is part of a larger alphanumeric sequence.
        assert!(!PackageRelease::tag_matches_commit("v1234567890", "1234567"));
        assert!(!PackageRelease::tag_matches_commit("release1234567build", "1234567"));

        // Version numbers that happen to contain similar digits.
        assert!(!PackageRelease::tag_matches_commit("v1.2.3", "123"));
        assert!(!PackageRelease::tag_matches_commit("v12.34.56", "234"));

        // Partial match without proper separator.
        assert!(!PackageRelease::tag_matches_commit("abc1234xyz", "abc1234"));
    }
}
