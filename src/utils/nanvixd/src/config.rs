// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration constants and utilities for Nanvix Daemon.
//!
//! This module provides default values used by the Nanvix Daemon.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::path::Path;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Fallback binary directory path for Nanvix binaries, used when the directory that contains the
/// running executable cannot be determined.
///
pub const FALLBACK_BIN_DIRECTORY: &str = "./bin";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the default directory in which the Nanvix Daemon looks for Nanvix binaries.
///
/// The default is the directory that contains the running executable. This way, a Nanvix Daemon
/// that is shipped alongside the binaries it depends on (e.g., in the `bin` directory of a release
/// archive) uses the co-located binaries, regardless of the current working directory. If that
/// directory cannot be determined, [`FALLBACK_BIN_DIRECTORY`] is returned instead.
///
/// Whether a symbolic link to the executable is followed depends on the platform:
/// [`std::env::current_exe()`] resolves it on Linux, but reports the path of the link itself on
/// Windows.
///
/// # Returns
///
/// The default binary directory path.
///
pub fn default_binary_directory() -> String {
    ::std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(executable_directory)
        .unwrap_or_else(|| FALLBACK_BIN_DIRECTORY.to_string())
}

///
/// # Description
///
/// Returns the directory that contains an executable.
///
/// # Parameters
///
/// - `executable_path`: Path to the executable.
///
/// # Returns
///
/// The directory that contains the executable, or `None` if `executable_path` has no parent
/// directory or if that directory is not valid UTF-8.
///
fn executable_directory(executable_path: &Path) -> Option<String> {
    executable_path
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .and_then(Path::to_str)
        .map(str::to_string)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::std::path::PathBuf;

    #[test]
    fn executable_directory_returns_parent_directory() {
        let bin_directory: PathBuf = PathBuf::from("sysroot").join("bin");
        let executable_path: PathBuf = bin_directory.join("nanvixd.elf");
        assert_eq!(executable_directory(&executable_path).as_deref(), bin_directory.to_str());
    }

    #[test]
    fn executable_directory_rejects_bare_file_name() {
        assert_eq!(executable_directory(Path::new("nanvixd.elf")), None);
    }

    #[test]
    fn executable_directory_rejects_root_directory() {
        assert_eq!(executable_directory(Path::new("/")), None);
    }

    #[cfg(unix)]
    #[test]
    fn executable_directory_rejects_non_utf8_directory() {
        use ::std::{
            ffi::OsStr,
            os::unix::ffi::OsStrExt,
        };

        let executable_path: &Path = Path::new(OsStr::from_bytes(b"/opt/\xff/nanvixd.elf"));
        assert_eq!(executable_directory(executable_path), None);
    }

    #[test]
    fn default_binary_directory_is_executable_directory() {
        let executable_path: PathBuf =
            ::std::env::current_exe().expect("failed to get current executable path");
        let expected: &Path = executable_path
            .parent()
            .expect("current executable has no parent directory");
        assert_eq!(Path::new(&default_binary_directory()), expected);
    }
}
