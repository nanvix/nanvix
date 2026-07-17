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
    /// Whether this entry is a directory.
    is_dir: bool,
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
    /// - `is_dir`: Whether this entry is a directory.
    /// - `size`: Size in bytes (0 for directories).
    pub fn new(name: String, is_dir: bool, size: u64) -> Self {
        Self { name, is_dir, size }
    }

    /// Returns the entry name (filename only, not full path).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether this entry is a directory.
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.is_dir
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
        let entry: DirEntry = DirEntry::new(String::from("test.txt"), false, 100);
        let cloned: DirEntry = entry.clone();
        assert_eq!(entry, cloned, "clone should preserve equality");

        assert_eq!(entry.name(), "test.txt", "name accessor should return name");
        assert!(!entry.is_dir(), "is_dir accessor should return false");
        assert_eq!(entry.size(), 100, "size accessor should return 100");

        let debug: alloc::string::String = alloc::format!("{entry:?}");
        assert!(debug.contains("test.txt"), "debug output should contain entry name");
    }
}
