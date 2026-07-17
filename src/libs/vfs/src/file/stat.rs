// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! File metadata.

//==================================================================================================
// Structures
//==================================================================================================

/// File metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    /// Size of the file in bytes (0 for directories).
    size: u64,
    /// Whether this is a directory.
    is_dir: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Stat {
    /// Creates a new `Stat` instance.
    ///
    /// # Parameters
    ///
    /// - `size`: File size in bytes (0 for directories).
    /// - `is_dir`: Whether this entry is a directory.
    pub fn new(size: u64, is_dir: bool) -> Self {
        Self { size, is_dir }
    }

    /// Returns the file size in bytes (0 for directories).
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns whether this entry is a directory.
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests Stat equality and debug.
    #[test]
    fn stat_clone_eq_debug() {
        let stat: Stat = Stat::new(42, false);
        let cloned: Stat = stat;
        assert_eq!(stat, cloned, "clone should preserve equality");

        let other: Stat = Stat::new(0, true);
        assert_ne!(stat, other, "different stats should not be equal");

        assert_eq!(stat.size(), 42, "size accessor should return 42");
        assert!(!stat.is_dir(), "is_dir accessor should return false");

        let debug: alloc::string::String = alloc::format!("{stat:?}");
        assert!(debug.contains("42"), "debug output should contain file size");
    }
}
