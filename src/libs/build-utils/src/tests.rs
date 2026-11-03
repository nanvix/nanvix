// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
