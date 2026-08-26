// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Directory entry metadata.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;

//==================================================================================================
// Structures
//==================================================================================================

/// Directory entry returned by [`super::read_dir()`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// Name of the entry (just the filename, not full path).
    name: String,
    /// Stable inode identifier.
    inode: u64,
    /// Whether this entry is a directory.
    is_dir: bool,
    /// Whether this entry is a character device.
    is_character_device: bool,
    /// Size in bytes (0 for directories).
    size: u64,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl DirEntry {
    /// Creates a new `DirEntry` instance.
    ///
    /// # Parameters
    ///
    /// - `name`: Entry name (filename only, not full path).
    /// - `inode`: Stable inode identifier.
    /// - `is_dir`: Whether this entry is a directory.
    /// - `size`: Size in bytes (0 for directories).
    pub fn new(name: String, inode: u64, is_dir: bool, size: u64) -> Self {
        Self {
            name,
            inode,
            is_dir,
            is_character_device: false,
            size,
        }
    }

    /// Creates a character-device directory entry.
    pub fn new_character_device(name: String, inode: u64) -> Self {
        Self {
            name,
            inode,
            is_dir: false,
            is_character_device: true,
            size: 0,
        }
    }

    /// Returns the entry name (filename only, not full path).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the stable inode identifier.
    #[must_use]
    pub fn inode(&self) -> u64 {
        self.inode
    }

    /// Returns whether this entry is a directory.
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Returns whether this entry is a character device.
    #[must_use]
    pub fn is_character_device(&self) -> bool {
        self.is_character_device
    }

    /// Returns the size in bytes (0 for directories).
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
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
}
