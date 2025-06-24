// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::file_access_mode::{
        O_EXEC,
        O_RDONLY,
        O_RDWR,
        O_WRONLY,
    },
    ffi::c_int,
};

//==================================================================================================
// FileAccessModeFlag
//==================================================================================================

/// A file access mode flag to be used with `open()`, `openat()`, and `fcntl()` system calls.
#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileAccessModeFlag {
    /// Set read-only access.
    Readonly = O_RDONLY,
    /// Set write-only access.
    WriteOnly = O_WRONLY,
    /// Set read-write access.
    ReadWrite = O_RDWR,
    /// Set execute-only access.
    ExecuteOnly = O_EXEC,
}

impl FileAccessModeFlag {
    /// Set search-only access.
    #[allow(non_upper_case_globals)]
    pub const SearchOnly: FileAccessModeFlag = Self::ExecuteOnly;
}

impl From<FileAccessModeFlag> for c_int {
    fn from(flag: FileAccessModeFlag) -> Self {
        flag as c_int
    }
}

//==================================================================================================
// FileAccessModeFlags
//==================================================================================================

///
/// # Description
///
/// File access mode flags to be used with `open()`, `openat()`, and `fcntl()` system calls.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileAccessModeFlags {
    flags: c_int,
}

impl FileAccessModeFlags {
    ///
    /// # Description
    ///
    /// Returns a bitmask of all file access mode flags.
    ///
    /// # Returns
    ///
    /// A bitmask of all file access mode flags.
    ///
    pub const fn mask() -> c_int {
        (FileAccessModeFlag::Readonly as c_int)
            | (FileAccessModeFlag::WriteOnly as c_int)
            | (FileAccessModeFlag::ReadWrite as c_int)
            | (FileAccessModeFlag::ExecuteOnly as c_int)
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileAccessModeFlags` instance with read-only access.
    ///
    /// # Returns
    ///
    /// A new `FileAccessModeFlags` instance with read-only access.
    ///
    pub const fn readonly() -> Self {
        FileAccessModeFlags {
            flags: FileAccessModeFlag::Readonly as c_int,
        }
    }

    /// # Description
    ///
    /// Creates a new `FileAccessModeFlags` instance with write-only access.
    ///
    /// # Returns
    ///
    /// A new `FileAccessModeFlags` instance with write-only access.
    ///
    pub const fn write_only() -> Self {
        FileAccessModeFlags {
            flags: FileAccessModeFlag::WriteOnly as c_int,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileAccessModeFlags` instance with read-write access.
    ///
    /// # Returns
    ///
    /// A new `FileAccessModeFlags` instance with read-write access.
    pub const fn read_write() -> Self {
        FileAccessModeFlags {
            flags: FileAccessModeFlag::ReadWrite as c_int,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileAccessModeFlags` instance with execute-only access.
    ///
    /// # Returns
    ///
    /// A new `FileAccessModeFlags` instance with execute-only access.
    ///
    pub const fn execute_only() -> Self {
        FileAccessModeFlags {
            flags: FileAccessModeFlag::ExecuteOnly as c_int,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileAccessModeFlags` instance with search-only access.
    ///
    /// # Returns
    ///
    /// A new `FileAccessModeFlags` instance with search-only access.
    ///
    pub const fn search_only() -> Self {
        FileAccessModeFlags {
            flags: FileAccessModeFlag::SearchOnly as c_int,
        }
    }

    ///
    /// # Description
    ///
    /// Checks if the access mode is read-only.
    ///
    /// # Returns
    ///
    /// `true` if the access mode is read-only, `false` otherwise.
    ///
    pub fn is_readonly(&self) -> bool {
        self.flags == FileAccessModeFlag::Readonly as c_int
    }

    ///
    /// # Description
    ///
    /// Checks if the access mode is write-only.
    ///
    /// # Returns
    ///
    /// `true` if the access mode is write-only, `false` otherwise.
    ///
    pub fn is_write_only(&self) -> bool {
        self.flags == FileAccessModeFlag::WriteOnly as c_int
    }

    ///
    /// # Description
    ///
    /// Checks if the access mode is read-write.
    ///
    /// # Returns
    ///
    /// `true` if the access mode is read-write, `false` otherwise.
    ///
    pub fn is_read_write(&self) -> bool {
        self.flags == FileAccessModeFlag::ReadWrite as c_int
    }

    ///
    /// # Description
    ///
    /// Checks if the access mode is execute-only.
    ///
    /// # Returns
    ///
    /// `true` if the access mode is execute-only, `false` otherwise.
    ///
    pub fn is_execute_only(&self) -> bool {
        self.flags == FileAccessModeFlag::ExecuteOnly as c_int
    }

    ///
    /// # Description
    ///
    /// Checks if the access mode is search-only.
    ///
    /// # Returns
    ///
    /// `true` if the access mode is search-only, `false` otherwise.
    ///
    pub fn is_search_only(&self) -> bool {
        self.flags == FileAccessModeFlag::SearchOnly as c_int
    }
}

impl Default for FileAccessModeFlags {
    fn default() -> Self {
        FileAccessModeFlags::readonly()
    }
}

impl From<FileAccessModeFlags> for c_int {
    fn from(flag: FileAccessModeFlags) -> Self {
        flag.flags
    }
}

impl From<&FileAccessModeFlags> for c_int {
    fn from(flag: &FileAccessModeFlags) -> Self {
        flag.flags
    }
}

impl TryFrom<c_int> for FileAccessModeFlags {
    type Error = Error;

    fn try_from(flags: c_int) -> Result<Self, Self::Error> {
        match flags {
            O_RDONLY => Ok(FileAccessModeFlags::readonly()),
            O_WRONLY => Ok(FileAccessModeFlags::write_only()),
            O_RDWR => Ok(FileAccessModeFlags::read_write()),
            O_EXEC => Ok(FileAccessModeFlags::execute_only()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "unsupported file access mode flags")),
        }
    }
}
