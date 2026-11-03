// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Backend-neutral filesystem metadata.

//==================================================================================================
// Structures
//==================================================================================================

/// File metadata returned by VFS operations.
///
/// Filesystem backends translate native metadata into this type before it is
/// converted to an outward-facing POSIX [`sysapi::sys_stat::stat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    /// Size of the file in bytes (0 for directories).
    size: u64,
    /// Whether this is a directory.
    is_dir: bool,
    /// Last access time (Unix seconds).
    atime: i64,
    /// Last modification time (Unix seconds).
    mtime: i64,
    /// Creation time (Unix seconds).
    ctime: i64,
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
    /// - `atime`: Last access time (Unix seconds).
    /// - `mtime`: Last modification time (Unix seconds).
    /// - `ctime`: Creation time (Unix seconds).
    pub fn new(size: u64, is_dir: bool, atime: i64, mtime: i64, ctime: i64) -> Self {
        Self {
            size,
            is_dir,
            atime,
            mtime,
            ctime,
        }
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

    /// Returns the last access time (Unix seconds).
    #[must_use]
    pub fn atime(&self) -> i64 {
        self.atime
    }

    /// Returns the last modification time (Unix seconds).
    #[must_use]
    pub fn mtime(&self) -> i64 {
        self.mtime
    }

    /// Returns the creation time (Unix seconds).
    #[must_use]
    pub fn ctime(&self) -> i64 {
        self.ctime
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;
