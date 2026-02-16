// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::{
    anyhow,
    Result,
};
use ::log::error;
use ::std::{
    collections::HashSet,
    path::{
        Component,
        Path,
        PathBuf,
    },
    process::ExitStatus,
};
use ::tokio::{
    fs,
    process::{
        Child,
        Command,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// .tar.bz2 file extension.
const TAR_BZ2_EXT: &str = ".tar.bz2";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents a tarball archive file.
///
#[derive(Debug)]
pub(crate) enum Tarball {
    /// Bzip2-compressed tarball.
    Bzip2 {
        /// Path to the tarball file.
        path: PathBuf,
    },
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Tarball {
    ///
    /// # Description
    ///
    /// Opens a tarball file and determines its compression format.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the tarball file.
    ///
    /// # Returns
    ///
    /// On success, returns a handle to the tarball file. On failure, it returns an object that
    /// describes the error.
    ///
    pub(crate) fn open(path: &Path) -> Result<Self> {
        // Check for supported formats, failing otherwise.
        if Tarball::is_bzip2(path) {
            // Open bzip2-compressed tarball.
            Ok(Tarball::Bzip2 {
                path: path.to_path_buf(),
            })
        } else {
            // Unsupported tarball format.
            let reason: String = format!("Unsupported tarball format: {}", path.to_string_lossy());
            error!("{reason}");
            anyhow::bail!(reason)
        }
    }

    ///
    /// # Description
    ///
    /// Checks whether the given filename corresponds to a supported tarball format.
    ///
    /// # Parameters
    ///
    /// - `filename`: Name of the file to check.
    ///
    /// # Returns
    ///
    /// This function returns `true` if the filename ends with a supported tarball extension,
    /// otherwise it returns `false`.
    ///
    pub(crate) fn is_supported(filename: &str) -> bool {
        filename.ends_with(TAR_BZ2_EXT)
    }

    ///
    /// # Description
    ///
    /// Extracts the tarball contents to the specified destination directory.
    ///
    /// # Parameters
    ///
    /// - `dest_dir`: The directory where the tarball contents will be extracted.
    ///
    /// # Returns
    ///
    /// On success, this function returns an empty tuple. On failure, it returns an object that
    /// describes the error.
    ///
    pub(crate) async fn extract(&self, dest_dir: &Path) -> Result<()> {
        match self {
            Tarball::Bzip2 { path } => extract_bzip2(path, dest_dir).await,
        }
    }

    ///
    /// # Description
    ///
    /// Checks whether the given path corresponds to a bzip2-compressed tarball.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the file to check.
    ///
    /// # Returns
    ///
    /// This function returns `true` if the path ends with `.tar.bz2`, otherwise `false`.
    ///
    fn is_bzip2(path: &Path) -> bool {
        path.to_string_lossy().ends_with(TAR_BZ2_EXT)
    }
}

///
/// # Description
///
/// Extracts a bzip2-compressed tarball using the `tar` command and validates that all extracted
/// paths remain within the destination directory to prevent tar slip attacks.
///
/// # Parameters
///
/// - `tarball_path`: Path to the tarball file.
/// - `dir`: The directory where the tarball contents will be extracted.
///
/// # Returns
///
/// On success, returns an empty tuple. On failure, it returns an object that describes the error.
///
async fn extract_bzip2(tarball_path: &Path, dir: &Path) -> anyhow::Result<()> {
    // Validate archive member paths before extraction to prevent tar slip attacks.
    validate_archive_paths(tarball_path).await?;

    // Spawn tar command.
    let mut child: Child = match Command::new("tar")
        .arg("-xjf")
        .arg(tarball_path)
        .arg("-C")
        .arg(dir)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let reason: String = format!("Failed to spawn tar command: {error}");
            error!("{reason}");
            anyhow::bail!(reason)
        },
    };

    // Wait for tar command to finish.
    let status: ExitStatus = match child.wait().await {
        Ok(status) => status,
        Err(error) => {
            let reason: String = format!("Failed to wait for tar command: {error}");
            error!("{reason}");
            return Err(anyhow!(reason));
        },
    };

    if !status.success() {
        let reason: String = "Tarball extraction failed".to_string();
        error!("{reason}");
        anyhow::bail!(reason)
    }

    // Validate that no extracted paths escape the destination directory.
    validate_extracted_paths(dir).await?;

    Ok(())
}

///
/// # Description
///
/// Validates archive member paths before extraction to reject entries containing absolute paths or
/// relative path components (e.g., `../../etc/malicious`) that could escape the destination
/// directory. This complements `validate_extracted_paths` by catching malicious entries before any
/// files are written to disk.
///
/// # Parameters
///
/// - `tarball_path`: Path to the tarball file to inspect.
///
/// # Returns
///
/// On success, returns an empty tuple. On failure, it returns an object that describes the error.
///
async fn validate_archive_paths(tarball_path: &Path) -> anyhow::Result<()> {
    // List archive contents without extracting.
    let output: ::std::process::Output = match Command::new("tar")
        .arg("-tjf")
        .arg(tarball_path)
        .output()
        .await
    {
        Ok(output) => output,
        Err(error) => {
            let reason: String = format!("Failed to list tarball contents: {error}");
            error!("{reason}");
            anyhow::bail!(reason)
        },
    };

    if !output.status.success() {
        let reason: String = "Failed to list tarball contents".to_string();
        error!("{reason}");
        anyhow::bail!(reason)
    }

    let contents: String = String::from_utf8_lossy(&output.stdout).to_string();

    for entry in contents.lines() {
        let path: &Path = Path::new(entry);

        // Reject absolute paths.
        if path.is_absolute() {
            let reason: String = format!("Archive contains absolute path: '{entry}'");
            error!("{reason}");
            anyhow::bail!(reason)
        }

        // Reject paths with parent directory components that could escape the destination.
        for component in path.components() {
            if component == Component::ParentDir {
                let reason: String = format!("Archive contains path traversal entry: '{entry}'");
                error!("{reason}");
                anyhow::bail!(reason)
            }
        }
    }

    Ok(())
}

///
/// # Description
///
/// Validates that all files and directories within the given directory are contained within it.
/// This prevents tar slip attacks where malicious archives extract files outside the intended
/// destination directory using relative path components (e.g., `../../etc/malicious`).
///
/// # Parameters
///
/// - `dir`: The directory to validate.
///
/// # Returns
///
/// On success, returns an empty tuple. On failure, it returns an object that describes the error.
///
async fn validate_extracted_paths(dir: &Path) -> anyhow::Result<()> {
    // Canonicalize the destination directory to resolve any symlinks or relative components.
    let canonical_dir: PathBuf = match fs::canonicalize(dir).await {
        Ok(path) => path,
        Err(error) => {
            let reason: String = format!(
                "Failed to canonicalize destination directory '{}': {error}",
                dir.display()
            );
            error!("{reason}");
            anyhow::bail!(reason)
        },
    };

    // Track visited canonical directories to prevent infinite loops from symlink cycles.
    let mut visited: HashSet<PathBuf> = HashSet::new();
    visited.insert(canonical_dir.clone());

    // Walk the directory tree iteratively and validate each path.
    let mut stack: Vec<PathBuf> = vec![canonical_dir.clone()];

    while let Some(current_dir) = stack.pop() {
        let mut read_dir: fs::ReadDir = match fs::read_dir(&current_dir).await {
            Ok(read_dir) => read_dir,
            Err(error) => {
                let reason: String =
                    format!("Failed to read directory '{}': {error}", current_dir.display());
                error!("{reason}");
                anyhow::bail!(reason)
            },
        };

        loop {
            match read_dir.next_entry().await {
                Ok(Some(entry)) => {
                    let path: PathBuf = entry.path();

                    // Canonicalize the entry path to resolve symlinks and relative components.
                    let canonical_path: PathBuf = match fs::canonicalize(&path).await {
                        Ok(path) => path,
                        Err(error) => {
                            let reason: String = format!(
                                "Failed to canonicalize path '{}': {error}",
                                path.display()
                            );
                            error!("{reason}");
                            anyhow::bail!(reason)
                        },
                    };

                    // Verify the canonical path is within the destination directory.
                    if !canonical_path.starts_with(&canonical_dir) {
                        let reason: String = format!(
                            "Extracted path '{}' escapes destination directory '{}'",
                            canonical_path.display(),
                            canonical_dir.display()
                        );
                        error!("{reason}");
                        anyhow::bail!(reason)
                    }

                    // If it is a directory, add it to the stack for further validation.
                    match fs::metadata(&canonical_path).await {
                        Ok(metadata) => {
                            if metadata.is_dir() && visited.insert(canonical_path.clone()) {
                                stack.push(canonical_path);
                            }
                        },
                        Err(error) => {
                            let reason: String = format!(
                                "Failed to read metadata for '{}': {error}",
                                path.display()
                            );
                            error!("{reason}");
                            anyhow::bail!(reason)
                        },
                    }
                },
                Ok(None) => {
                    break;
                },
                Err(error) => {
                    let reason: String = format!(
                        "Failed to read directory entry in '{}': {error}",
                        current_dir.display()
                    );
                    error!("{reason}");
                    anyhow::bail!(reason)
                },
            }
        }
    }

    Ok(())
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
    /// Tests that `.tar.bz2` files are recognized as supported.
    ///
    #[test]
    fn test_is_supported_tar_bz2() {
        assert!(Tarball::is_supported("file.tar.bz2"));
        assert!(Tarball::is_supported("archive.tar.bz2"));
        assert!(Tarball::is_supported("nanvix-release.tar.bz2"));
    }

    ///
    /// # Description
    ///
    /// Tests that unsupported formats are not recognized.
    ///
    #[test]
    fn test_is_supported_unsupported() {
        assert!(!Tarball::is_supported("file.tar.gz"));
        assert!(!Tarball::is_supported("file.zip"));
        assert!(!Tarball::is_supported("file.tar"));
        assert!(!Tarball::is_supported("file.bz2"));
        assert!(!Tarball::is_supported("file.txt"));
    }

    ///
    /// # Description
    ///
    /// Tests that bzip2 format is correctly identified.
    ///
    #[test]
    fn test_is_bzip2() {
        let path: PathBuf = PathBuf::from("/tmp/test.tar.bz2");
        assert!(Tarball::is_bzip2(&path));

        let path2: PathBuf = PathBuf::from("/tmp/test.tar.gz");
        assert!(!Tarball::is_bzip2(&path2));
    }

    ///
    /// # Description
    ///
    /// Tests that opening unsupported tarball format returns an error.
    ///
    #[test]
    fn test_open_unsupported() {
        let path: PathBuf = PathBuf::from("/tmp/test.tar.gz");
        let result: Result<Tarball> = Tarball::open(&path);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("Unsupported tarball format"));
    }

    ///
    /// # Description
    ///
    /// Tests that opening a bzip2 tarball succeeds.
    ///
    #[test]
    fn test_open_bzip2() {
        let path: PathBuf = PathBuf::from("/tmp/test.tar.bz2");
        let result: Result<Tarball> = Tarball::open(&path);
        assert!(result.is_ok());

        match result.expect("failed") {
            Tarball::Bzip2 { path: p } => {
                assert_eq!(p, path);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Tests path extraction from tarball.
    ///
    #[test]
    fn test_tarball_path() {
        let expected_path: PathBuf = PathBuf::from("/tmp/archive.tar.bz2");
        let tarball: Tarball = Tarball::Bzip2 {
            path: expected_path.clone(),
        };

        match tarball {
            Tarball::Bzip2 { path } => {
                assert_eq!(path, expected_path);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Creates a unique temporary directory name to avoid conflicts when tests run concurrently.
    ///
    fn unique_temp_dir(prefix: &str) -> PathBuf {
        ::std::env::temp_dir().join(format!(
            "{}-{}-{}",
            prefix,
            ::std::process::id(),
            ::std::time::SystemTime::now()
                .duration_since(::std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    ///
    /// # Description
    ///
    /// Tests that path validation succeeds for a directory with only contained files.
    ///
    #[tokio::test]
    async fn test_validate_extracted_paths_valid() {
        let dir: PathBuf = unique_temp_dir("tarball_test_valid");
        let _ = fs::remove_dir_all(&dir).await;
        fs::create_dir_all(dir.join("subdir"))
            .await
            .expect("failed");
        fs::write(dir.join("file.txt"), "content")
            .await
            .expect("failed");
        fs::write(dir.join("subdir/nested.txt"), "content")
            .await
            .expect("failed");

        let result: Result<()> = validate_extracted_paths(&dir).await;
        assert!(result.is_ok());

        let _ = fs::remove_dir_all(&dir).await;
    }

    ///
    /// # Description
    ///
    /// Tests that path validation fails when a symlink escapes the destination directory.
    ///
    #[tokio::test]
    async fn test_validate_extracted_paths_symlink_escape() {
        let base: PathBuf = unique_temp_dir("tarball_test_escape");
        let dir: PathBuf = base.join("dest");
        let outside: PathBuf = base.join("outside");
        let _ = fs::remove_dir_all(&base).await;
        fs::create_dir_all(&dir).await.expect("failed");
        fs::create_dir_all(&outside).await.expect("failed");
        fs::write(outside.join("secret.txt"), "secret")
            .await
            .expect("failed");

        // Create a symlink inside `dir` that points to a file outside `dir`.
        fs::symlink(outside.join("secret.txt"), dir.join("escape_link"))
            .await
            .expect("failed");

        let result: Result<()> = validate_extracted_paths(&dir).await;
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("escapes destination directory"));

        let _ = fs::remove_dir_all(&base).await;
    }

    ///
    /// # Description
    ///
    /// Tests that path validation succeeds for an empty directory.
    ///
    #[tokio::test]
    async fn test_validate_extracted_paths_empty_dir() {
        let dir: PathBuf = unique_temp_dir("tarball_test_empty");
        let _ = fs::remove_dir_all(&dir).await;
        fs::create_dir_all(&dir).await.expect("failed");

        let result: Result<()> = validate_extracted_paths(&dir).await;
        assert!(result.is_ok());

        let _ = fs::remove_dir_all(&dir).await;
    }
}
