// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Descriptor-level file metadata.

//==================================================================================================
// Structures
//==================================================================================================

/// File metadata returned by stat operations.
///
/// This is the VFS-level metadata type, independent of any concrete
/// filesystem. Backend modules translate their native metadata into this
/// type.
pub struct VfsStat {
    /// File size in bytes (0 for directories).
    size: u64,
    /// Whether this entry is a directory.
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

impl VfsStat {
    /// Creates a new `VfsStat`.
    pub fn new(size: u64, is_dir: bool, atime: i64, mtime: i64, ctime: i64) -> Self {
        Self {
            size,
            is_dir,
            atime,
            mtime,
            ctime,
        }
    }

    /// Returns the file size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns whether this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Returns the last access time (Unix seconds).
    pub fn atime(&self) -> i64 {
        self.atime
    }

    /// Returns the last modification time (Unix seconds).
    pub fn mtime(&self) -> i64 {
        self.mtime
    }

    /// Returns the creation time (Unix seconds).
    pub fn ctime(&self) -> i64 {
        self.ctime
    }
}
