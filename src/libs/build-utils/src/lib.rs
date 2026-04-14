// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Finds the workspace root by walking up from `CARGO_MANIFEST_DIR` until a `Cargo.toml`
/// containing a `[workspace]` section is found.
///
/// # Returns
///
/// The absolute path to the workspace root directory.
///
pub fn find_workspace_root() -> PathBuf {
    let manifest_dir: &str = env!("CARGO_MANIFEST_DIR");
    let mut current: &Path = Path::new(manifest_dir);

    loop {
        let cargo_toml: PathBuf = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content: String =
                fs::read_to_string(&cargo_toml).expect("failed to read Cargo.toml");
            if content.contains("[workspace]") {
                return current.to_path_buf();
            }
        }

        current = current
            .parent()
            .unwrap_or_else(|| panic!("failed to find workspace root from '{}'", manifest_dir));
    }
}

///
/// # Description
///
/// Returns the `memory_size` value (in bytes) from the `MEMORY_SIZE_BYTES` environment variable
/// exported by the Makefile.
///
/// # Returns
///
/// The memory size in bytes.
///
pub fn memory_size() -> usize {
    let val: String = ::std::env::var("MEMORY_SIZE_BYTES")
        .expect("MEMORY_SIZE_BYTES env var is required (set MEMORY_SIZE in the Makefile)");
    val.parse()
        .unwrap_or_else(|_| panic!("Invalid MEMORY_SIZE_BYTES value: '{val}'"))
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_find_workspace_root() {
        let root: PathBuf = find_workspace_root();
        assert!(root.join("Cargo.toml").exists());
        let content: String = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(content.contains("[workspace]"));
    }

    #[test]
    fn test_memory_size() {
        let size: usize = memory_size();
        // The value must be positive and a multiple of 1 MB.
        assert!(size > 0, "memory_size should be greater than zero");
        assert_eq!(size % (1024 * 1024), 0, "memory_size should be a multiple of 1 MB");
    }
}
