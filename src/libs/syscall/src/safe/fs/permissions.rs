// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use ::alloc::string::String;
use ::core::fmt;
use ::sysapi::{
    sys_stat::file_mode::{
        S_IRGRP,
        S_IROTH,
        S_IRUSR,
        S_IRWXG,
        S_IRWXO,
        S_IRWXU,
        S_IWGRP,
        S_IWOTH,
        S_IWUSR,
        S_IXGRP,
        S_IXOTH,
        S_IXUSR,
    },
    sys_types::mode_t,
};

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
    /// Sets or clears user read permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with user read permission set or cleared.
    ///
    pub fn user_read(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IRUSR;
        } else {
            self.0 &= !S_IRUSR;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Sets or clears user write permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with user write permission set or cleared.
    ///
    pub fn user_write(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IWUSR;
        } else {
            self.0 &= !S_IWUSR;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Sets or clears user execute permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with user execute permission set or cleared.
    ///
    pub fn user_execute(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IXUSR;
        } else {
            self.0 &= !S_IXUSR;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Sets or clears group read permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with group read permission set or cleared.
    ///
    pub fn group_read(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IRGRP;
        } else {
            self.0 &= !S_IRGRP;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Sets or clears group write permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with group write permission set or cleared.
    ///
    pub fn group_write(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IWGRP;
        } else {
            self.0 &= !S_IWGRP;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Sets or clears group execute permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with group execute permission set or cleared.
    ///
    pub fn group_execute(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IXGRP;
        } else {
            self.0 &= !S_IXGRP;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Sets or clears others read permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with others read permission set or cleared.
    ///
    pub fn others_read(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IROTH;
        } else {
            self.0 &= !S_IROTH;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Sets or clears others write permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with others write permission set or cleared.
    ///
    pub fn others_write(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IWOTH;
        } else {
            self.0 &= !S_IWOTH;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Sets or clears others execute permission stored in `self`.
    ///
    /// # Arguments
    ///
    /// * `enable` - If `true`, enables the permission; if `false`, disables it.
    ///
    /// # Returns
    ///
    /// A new `FileSystemPermissions` structure with others execute permission set or cleared.
    ///
    pub fn others_execute(mut self, enable: bool) -> Self {
        if enable {
            self.0 |= S_IXOTH;
        } else {
            self.0 &= !S_IXOTH;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Checks if the user has read permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if the user has read permission, otherwise `false`.
    ///
    pub fn user_can_read(&self) -> bool {
        self.0 & S_IRUSR != 0
    }

    ///
    /// # Description
    ///
    /// Checks if the user has write permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if the user has write permission, otherwise `false`.
    ///
    pub fn user_can_write(&self) -> bool {
        self.0 & S_IWUSR != 0
    }

    ///
    /// # Description
    ///
    /// Checks if the user has execute permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if the user has execute permission, otherwise `false`.
    ///
    pub fn user_can_execute(&self) -> bool {
        self.0 & S_IXUSR != 0
    }

    ///
    /// # Description
    ///
    /// Checks if the group has read permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if the group has read permission, otherwise `false`.
    ///
    pub fn group_can_read(&self) -> bool {
        self.0 & S_IRGRP != 0
    }

    ///
    /// # Description
    ///
    /// Checks if the group has write permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if the group has write permission, otherwise `false`.
    ///
    pub fn group_can_write(&self) -> bool {
        self.0 & S_IWGRP != 0
    }

    ///
    /// # Description
    ///
    /// Checks if the group has execute permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if the group has execute permission, otherwise `false`.
    ///
    pub fn group_can_execute(&self) -> bool {
        self.0 & S_IXGRP != 0
    }

    ///
    /// # Description
    ///
    /// Checks if others have read permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if others have read permission, otherwise `false`.
    ///
    pub fn others_can_read(&self) -> bool {
        self.0 & S_IROTH != 0
    }

    ///
    /// # Description
    ///
    /// Checks if others have write permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if others have write permission, otherwise `false`.
    ///
    pub fn others_can_write(&self) -> bool {
        self.0 & S_IWOTH != 0
    }

    ///
    /// # Description
    ///
    /// Checks if others have execute permission.
    ///
    /// # Returns
    ///
    /// Returns `true` if others have execute permission, otherwise `false`.
    ///
    pub fn others_can_execute(&self) -> bool {
        self.0 & S_IXOTH != 0
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
        let user: [char; 3] = to_rwx(S_IRUSR, S_IWUSR, S_IXUSR);
        let group: [char; 3] = to_rwx(S_IRGRP, S_IWGRP, S_IXGRP);
        let other: [char; 3] = to_rwx(S_IROTH, S_IWOTH, S_IXOTH);

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
        self.0 & S_IRWXU == other.0 & S_IRWXU
            && self.0 & S_IRWXG == other.0 & S_IRWXG
            && self.0 & S_IRWXO == other.0 & S_IRWXO
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
