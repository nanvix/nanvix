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
/// Returns the `memory_size` value (in bytes) from `build/kernel_config.toml`.
/// The file is located relative to the workspace root. Both decimal and `0x`-prefixed hexadecimal
/// values are accepted.
///
/// # Returns
///
/// The memory size in bytes.
///
pub fn memory_size() -> usize {
    let root = find_workspace_root();
    let toml_path = root.join("build/kernel_config.toml");
    let contents = fs::read_to_string(&toml_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", toml_path.display()));
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("memory_size") {
            let rest = rest
                .trim()
                .strip_prefix('=')
                .expect("malformed memory_size line")
                .trim();
            return if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
                usize::from_str_radix(hex, 16).expect("bad hex memory_size")
            } else {
                rest.parse().expect("bad decimal memory_size")
            };
        }
    }
    panic!("memory_size not found in {}", toml_path.display());
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
