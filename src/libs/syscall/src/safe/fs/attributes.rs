// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::safe::{
    time::Time,
    FileSystemPermissions,
    FileType,
    RegularFileOffset,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::sys_stat;

//==================================================================================================
// File System Attributes
//==================================================================================================

///
/// # Description
///
/// A structure that represents the attributes of a file in the file system.
///
#[derive(Debug, Clone, Copy)]
pub struct FileSystemAttributes(sys_stat::stat);

impl FileSystemAttributes {
    ///
    /// # Description
    ///
    /// Returns the last access time of the file stored in `self`.
    ///
    /// # Returns
    ///
    /// The last access time of the file stored in `self`.
    ///
    pub fn accessed(&self) -> Result<Time, Error> {
        Time::try_from(self.0.st_atim)
    }

    ///
    /// # Description
    ///
    /// Returns the last creation time of the file stored in `self`.
    ///
    /// # Returns
    ///
    /// The last creation time of the file stored in `self`.
    ///
    pub fn created(&self) -> Result<Time, Error> {
        Err(Error::new(ErrorCode::OperationNotSupported, "creation time not supported"))
    }

    ///
    /// # Description
    ///
    /// Creates an empty `FileSystemAttributes` structure.
    ///
    pub fn empty() -> Self {
        FileSystemAttributes(sys_stat::stat::default())
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
    /// Returns the last modification time of the file stored in `self`.
    ///
    /// # Returns
    ///
    /// The last modification time of the file stored in `self`.
    ///
    pub fn modified(&self) -> Result<Time, Error> {
        Time::try_from(self.0.st_mtim)
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
    /// Casts `self` to a raw mutable `stat::stat` structure.
    ///
    /// # Returns
    ///
    /// A mutable reference to the raw `stat::stat` structure.
    ///
    pub fn as_raw_mut(&mut self) -> &mut sys_stat::stat {
        &mut self.0
    }

    ///
    /// # Description
    ///
    /// Casts `self` to a raw `stat::stat` structure.
    ///
    /// # Returns
    ///
    /// A reference to the raw `stat::stat` structure.
    ///
    pub fn as_raw(&self) -> &sys_stat::stat {
        &self.0
    }
}

impl From<sys_stat::stat> for FileSystemAttributes {
    fn from(stat: sys_stat::stat) -> FileSystemAttributes {
        FileSystemAttributes(stat)
    }
}

impl From<FileSystemAttributes> for sys_stat::stat {
    fn from(attributes: FileSystemAttributes) -> sys_stat::stat {
        attributes.0
    }
}
