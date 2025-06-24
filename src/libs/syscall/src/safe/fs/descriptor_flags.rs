// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::c_int;
use sysapi::fcntl::file_descriptor_flags::{
    FD_CLOEXEC,
    FD_CLOFORK,
};

//==================================================================================================
// FileDescriptorFlag
//==================================================================================================

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileDescriptorFlag {
    /// Close-on-exec flag.
    CloseOnExec = FD_CLOEXEC,
    /// Close-on-fork flag.
    CloseOnFork = FD_CLOFORK,
}

impl From<FileDescriptorFlag> for c_int {
    fn from(flag: FileDescriptorFlag) -> Self {
        flag as c_int
    }
}

//==================================================================================================
// FileDescriptorFlags
//==================================================================================================

///
/// # Description
///
/// File descriptor flags to be used with `fcntl()` system calls.
///
#[derive(Debug)]
pub struct FileDescriptorFlags {
    flags: c_int,
}

impl FileDescriptorFlags {
    /// Creates a new `FileDescriptorFlags` instance with no flags set.
    const fn empty() -> Self {
        FileDescriptorFlags { flags: 0 }
    }

    /// Creates a new `FileDescriptorFlags` instance with all flags set.
    const fn all_flags() -> Self {
        FileDescriptorFlags {
            flags: (FileDescriptorFlag::CloseOnExec as c_int)
                | (FileDescriptorFlag::CloseOnFork as c_int),
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileDescriptorFlags` instance with the close-on-exec flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the close-on-exec flag is set; if `false`, the close-on-exec flag
    ///   is unset.
    ///
    /// # Returns
    ///
    /// A new `FileDescriptorFlags` instance with the close-on-exec flag set or unset.
    ///
    pub fn set_close_on_exec(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileDescriptorFlag::CloseOnExec.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileDescriptorFlag::CloseOnExec.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileDescriptorFlags` instance with the close-on-fork flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the close-on-fork flag is set; if `false`, the close-on-fork flag
    ///   is unset.
    ///
    /// # Returns
    ///
    /// A new `FileDescriptorFlags` instance with the close-on-fork flag set or unset.
    ///
    pub fn set_close_on_fork(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileDescriptorFlag::CloseOnFork.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileDescriptorFlag::CloseOnFork.into();
            self.flags &= !flag;
        }
        self
    }
}

impl Default for FileDescriptorFlags {
    fn default() -> Self {
        FileDescriptorFlags::empty()
    }
}

impl From<FileDescriptorFlags> for c_int {
    fn from(flag: FileDescriptorFlags) -> Self {
        flag.flags
    }
}

impl From<&FileDescriptorFlags> for c_int {
    fn from(flag: &FileDescriptorFlags) -> Self {
        flag.flags
    }
}

impl TryFrom<c_int> for FileDescriptorFlags {
    type Error = Error;

    fn try_from(flags: c_int) -> Result<Self, Self::Error> {
        // Check if any unsupported flags is set.
        if flags & !FileDescriptorFlags::all_flags().flags != 0 {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "unsupported file descriptor flags",
            ));
        }

        let mut descriptor_flags: FileDescriptorFlags = FileDescriptorFlags::empty();

        // Set close-on-exec flag.
        if flags & FileDescriptorFlag::CloseOnExec as c_int != 0 {
            descriptor_flags = descriptor_flags.set_close_on_exec(true);
        }

        // Set close-on-fork flag.
        if flags & FileDescriptorFlag::CloseOnFork as c_int != 0 {
            descriptor_flags = descriptor_flags.set_close_on_fork(true);
        }

        Ok(descriptor_flags)
    }
}
