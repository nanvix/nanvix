// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests DirEntry equality and debug.
#[test]
fn dir_entry_clone_eq_debug() {
    let entry: DirEntry = DirEntry::new(String::from("test.txt"), 7, false, 100);
    let cloned: DirEntry = entry.clone();
    assert_eq!(entry, cloned, "clone should preserve equality");

    assert_eq!(entry.name(), "test.txt", "name accessor should return name");
    assert_eq!(entry.inode(), 7, "inode accessor should return inode");
    assert!(!entry.is_dir(), "is_dir accessor should return false");
    assert_eq!(entry.size(), 100, "size accessor should return 100");

    let debug: alloc::string::String = alloc::format!("{entry:?}");
    assert!(debug.contains("test.txt"), "debug output should contain entry name");
}
