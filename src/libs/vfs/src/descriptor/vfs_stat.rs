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
}

//==================================================================================================
// Implementations
//==================================================================================================

impl VfsStat {
    /// Creates a new `VfsStat`.
    pub fn new(size: u64, is_dir: bool) -> Self {
        Self { size, is_dir }
    }

    /// Returns the file size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns whether this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
}
