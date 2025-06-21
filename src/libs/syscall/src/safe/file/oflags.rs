// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::fcntl::{
    self,
};
use ::sysapi::{
    fcntl::file_access_mode::O_EXEC,
    ffi::c_int,
};

//==================================================================================================
// OpenOptions
//==================================================================================================

///
/// # Description
///
/// A structure that represents the options for opening a regular file.
///
#[derive(Debug)]
pub struct RegularFileOpenFlags(c_int);

impl RegularFileOpenFlags {
    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with execute-only permissions.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with execute-only permissions.
    ///
    pub fn execute_only() -> Self {
        RegularFileOpenFlags(O_EXEC)
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with read-only permissions.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with read-only permissions.
    ///
    pub fn read_only() -> Self {
        RegularFileOpenFlags(fcntl::OpenFlags::Readonly as i32)
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with read-write permissions.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with read-write permissions.
    ///
    pub fn read_write() -> Self {
        RegularFileOpenFlags(fcntl::OpenFlags::ReadWrite as i32)
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with write-only permissions.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with write-only permissions.
    ///
    pub fn write_only() -> Self {
        RegularFileOpenFlags(fcntl::OpenFlags::WriteOnly as i32)
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with the append flag set.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with the append flag set.
    ///
    pub fn append(mut self) -> Self {
        self.0 |= fcntl::OpenFlags::Append as i32;
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with the create flag set.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with the create flag set.
    ///
    pub fn create(mut self) -> Self {
        self.0 |= fcntl::OpenFlags::Create as i32;
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with the exclusive flag set.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with the exclusive flag set.
    ///
    pub fn exclusive(mut self) -> Self {
        self.0 |= fcntl::OpenFlags::Exclusive as i32;
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with the truncate flag set.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with the truncate flag set.
    ///
    pub fn truncate(mut self) -> Self {
        self.0 |= fcntl::OpenFlags::Truncate as i32;
        self
    }

    ///
    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with sync flag set.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with the sync flag set.
    ///
    pub fn sync(mut self) -> Self {
        self.0 |= fcntl::OpenFlags::Sync as i32;
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `RegularFileOpenFlags` with the non-blocking flag set.
    ///
    /// # Returns
    ///
    /// A new `RegularFileOpenFlags` instance with the non-blocking flag set.
    ///
    pub fn non_blocking(mut self) -> Self {
        self.0 |= fcntl::OpenFlags::NonBlocking as i32;
        self
    }
}

impl From<RegularFileOpenFlags> for c_int {
    fn from(flags: RegularFileOpenFlags) -> c_int {
        flags.0
    }
}
