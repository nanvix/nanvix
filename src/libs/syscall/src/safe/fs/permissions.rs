// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::{
    fcntl::{
        self,
        S_IRUSR,
        S_IWUSR,
    },
    sys::types::mode_t,
};
use ::alloc::string::String;
use ::core::fmt;

//==================================================================================================
// File Permissions
//==================================================================================================

///
/// # Description
///
/// A structure that represents the permissions of a file in the file system.
///
#[derive(Default, Clone, Copy)]
pub struct FileSystemPermissions(mode_t);

impl FileSystemPermissions {
    ///
    /// # Description
    ///
    /// Creates an empty `FileSystemPermissions` structure.
    ///
    pub fn empty() -> Self {
        FileSystemPermissions::default()
    }

    ///
    /// # Description
    ///
    /// Enables user read permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with user read permission enabled.
    ///
    pub fn user_read(mut self) -> Self {
        self.0 |= S_IRUSR;
        self
    }

    ///
    /// # Description
    ///
    /// Enables user write permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with user write permission enabled.
    ///
    pub fn user_write(mut self) -> Self {
        self.0 |= S_IWUSR;
        self
    }

    ///
    /// # Description
    ///
    /// Enables user execute permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with user execute permission enabled.
    ///
    pub fn user_execute(mut self) -> Self {
        self.0 |= fcntl::S_IXUSR;
        self
    }

    ///
    /// # Description
    ///
    /// Enables group read permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with group read permission enabled.
    ///
    pub fn group_read(mut self) -> Self {
        self.0 |= fcntl::S_IRGRP;
        self
    }

    ///
    /// # Description
    ///
    /// Enables group write permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with group write permission enabled.
    ///
    pub fn group_write(mut self) -> Self {
        self.0 |= fcntl::S_IWGRP;
        self
    }

    ///
    /// # Description
    ///
    /// Enables group execute permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with group execute permission enabled.
    ///
    pub fn group_execute(mut self) -> Self {
        self.0 |= fcntl::S_IXGRP;
        self
    }

    ///
    /// # Description
    ///
    /// Enables others read permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with others read permission enabled.
    ///
    pub fn others_read(mut self) -> Self {
        self.0 |= fcntl::S_IROTH;
        self
    }

    ///
    /// # Description
    ///
    /// Enables others write permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with others write permission enabled.
    ///
    pub fn others_write(mut self) -> Self {
        self.0 |= fcntl::S_IWOTH;
        self
    }

    ///
    /// # Description
    ///
    /// Enables others execute permission stored in `self`.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with others execute permission enabled.
    ///
    pub fn others_execute(mut self) -> Self {
        self.0 |= fcntl::S_IXOTH;
        self
    }
}

impl fmt::Debug for FileSystemPermissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode: mode_t = self.0;
        let to_rwx = |read: mode_t, write: mode_t, exec: mode_t| -> [char; 3] {
            [
                if mode & read != 0 { 'r' } else { '-' },
                if mode & write != 0 { 'w' } else { '-' },
                if mode & exec != 0 { 'x' } else { '-' },
            ]
        };
        let user: [char; 3] = to_rwx(fcntl::S_IRUSR, fcntl::S_IWUSR, fcntl::S_IXUSR);
        let group: [char; 3] = to_rwx(fcntl::S_IRGRP, fcntl::S_IWGRP, fcntl::S_IXGRP);
        let other: [char; 3] = to_rwx(fcntl::S_IROTH, fcntl::S_IWOTH, fcntl::S_IXOTH);

        write!(
            f,
            "FileSystemPermissions({}{}{})",
            user.iter().collect::<String>(),
            group.iter().collect::<String>(),
            other.iter().collect::<String>()
        )
    }
}

impl PartialEq for FileSystemPermissions {
    fn eq(&self, other: &Self) -> bool {
        self.0 & fcntl::S_IRWXU == other.0 & fcntl::S_IRWXU
            && self.0 & fcntl::S_IRWXG == other.0 & fcntl::S_IRWXG
            && self.0 & fcntl::S_IRWXO == other.0 & fcntl::S_IRWXO
    }
}

impl Eq for FileSystemPermissions {}

impl From<FileSystemPermissions> for mode_t {
    fn from(permissions: FileSystemPermissions) -> mode_t {
        permissions.0
    }
}

impl From<mode_t> for FileSystemPermissions {
    fn from(permissions: mode_t) -> FileSystemPermissions {
        FileSystemPermissions(permissions)
    }
}
