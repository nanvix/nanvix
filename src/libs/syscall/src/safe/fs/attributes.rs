// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::{
    safe::{
        FileSystemPermissions,
        FileType,
        RegularFileOffset,
    },
    sys::stat::{
        self,
    },
};

//==================================================================================================
// File System Attributes
//==================================================================================================

///
/// # Description
///
/// A structure that represents the attributes of a file in the file system.
///
#[derive(Debug)]
pub struct FileSystemAttributes(stat::stat);

impl FileSystemAttributes {
    ///
    /// # Description
    ///
    /// Creates an empty `FileSystemAttributes` structure.
    ///
    pub fn empty() -> Self {
        FileSystemAttributes(stat::stat::default())
    }

    ///
    /// # Description
    ///
    /// Returns the filze size in bytes stored in `self`
    ///
    /// # Returns
    ///
    /// The the file size stored in `self`.
    ///
    pub fn size(&self) -> RegularFileOffset {
        RegularFileOffset::from(self.0.st_size)
    }

    ///
    /// # Description
    ///
    /// Returns the file type stored in `self`.
    ///
    /// # Returns
    ///
    /// The file type stored in `self`.
    ///
    pub fn file_type(&self) -> FileType {
        FileType::from(self.0.st_mode)
    }

    ///
    /// # Description
    ///
    /// Returns the file permissions stored in `self`.
    ///
    /// # Returns
    ///
    /// The file permissions stored in `self`.
    ///
    pub fn permissions(&self) -> FileSystemPermissions {
        FileSystemPermissions::from(self.0.st_mode)
    }

    ///
    /// # Description
    ///
    /// Casts `self` to a raw `stat::stat` structure.
    ///
    /// # Returns
    ///
    /// A mutable reference to the raw `stat::stat` structure.
    ///
    pub fn as_raw_mut(&mut self) -> &mut stat::stat {
        &mut self.0
    }
}

impl From<stat::stat> for FileSystemAttributes {
    fn from(stat: stat::stat) -> FileSystemAttributes {
        FileSystemAttributes(stat)
    }
}

impl From<FileSystemAttributes> for stat::stat {
    fn from(attributes: FileSystemAttributes) -> stat::stat {
        attributes.0
    }
}
