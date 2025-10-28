// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::{
    anyhow,
    Result,
};
use ::std::{
    path::{
        Path,
        PathBuf,
    },
    process::ExitStatus,
};
use ::syslog::error;
use ::tokio::process::{
    Child,
    Command,
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
    pub(crate) async fn extract(&self, dest_dir: &PathBuf) -> Result<()> {
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
/// Extracts a bzip2-compressed tarball using the `tar` command.
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
async fn extract_bzip2(tarball_path: &PathBuf, dir: &PathBuf) -> anyhow::Result<()> {
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
            let reason: String = format!("Failed to spawn tar command: {}", error);
            error!("{reason}");
            anyhow::bail!(reason)
        },
    };

    // Wait for tar command to finish.
    let status: ExitStatus = match child.wait().await {
        Ok(status) => status,
        Err(error) => {
            let reason: String = format!("Failed to wait for tar command: {}", error);
            error!("{reason}");
            return Err(anyhow!(reason));
        },
    };

    if !status.success() {
        let reason: String = "Tarball extraction failed".to_string();
        error!("{reason}");
        anyhow::bail!(reason)
    }

    Ok(())
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
            .unwrap_err()
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

        match result.unwrap() {
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
}
