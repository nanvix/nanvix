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
    fcntl::file_status_flags::{
        O_APPEND,
        O_NONBLOCK,
        O_SYNC,
    },
    ffi::c_int,
};

//==================================================================================================
// FileStatusFlag
//==================================================================================================

/// A file status flag to be used with `open()`, `openat()`, and `fcntl()` system calls.
#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FileStatusFlag {
    /// Set append mode.
    Append = O_APPEND,
    /// Synchronized I/O operations.
    SynchronizedIo = O_SYNC,
    /// Non-blocking mode.
    NonBlocking = O_NONBLOCK,
}

impl From<FileStatusFlag> for i32 {
    fn from(flag: FileStatusFlag) -> Self {
        flag as c_int
    }
}

//==================================================================================================
// FileStatusFlags
//==================================================================================================

///
/// # Description
///
/// File status flags to be used with `open()`, `openat()`, and `fcntl()` system calls.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileStatusFlags {
    flags: c_int,
}

impl FileStatusFlags {
    /// Creates a new `FileStatusFlags` instance with no flags set.
    const fn empty() -> Self {
        FileStatusFlags { flags: 0 }
    }

    ///
    /// # Description
    ///
    /// Returns a bitmask of all valid file status flags.
    ///
    /// # Returns
    ///
    /// A bitmask of all valid file status flags.
    ///
    pub const fn mask() -> c_int {
        (FileStatusFlag::Append as c_int)
            | (FileStatusFlag::SynchronizedIo as c_int)
            | (FileStatusFlag::NonBlocking as c_int)
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileStatusFlags` instance with the append flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the append flag is set; if `false`, the append flag is unset.
    ///
    /// # Returns
    ///
    /// A new `FileStatusFlags` instance with the append flag set or unset.
    ///
    pub fn set_append(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileStatusFlag::Append.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileStatusFlag::Append.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileStatusFlags` instance with the non-blocking flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the non-blocking flag is set; if `false`, the non-blocking flag is
    ///   unset.
    ///
    /// # Returns
    ///
    /// A new `FileStatusFlags` instance with the non-blocking flag set or unset.
    ///
    pub fn set_non_blocking(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileStatusFlag::NonBlocking.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileStatusFlag::NonBlocking.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileStatusFlags` instance with the synchronized I/O flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the synchronized I/O flag is set; if `false`, the synchronized I/O
    ///   flag is unset.
    ///
    /// # Returns
    ///
    /// A new `FileStatusFlags` instance with the synchronized I/O flag set or unset.
    ///
    pub fn set_synchronized_io(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileStatusFlag::SynchronizedIo.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileStatusFlag::SynchronizedIo.into();
            self.flags &= !flag;
        }
        self
    }
}

impl Default for FileStatusFlags {
    fn default() -> Self {
        FileStatusFlags::empty()
    }
}

impl From<FileStatusFlags> for c_int {
    fn from(flag: FileStatusFlags) -> Self {
        flag.flags
    }
}

impl From<&FileStatusFlags> for c_int {
    fn from(flag: &FileStatusFlags) -> Self {
        flag.flags
    }
}

impl TryFrom<c_int> for FileStatusFlags {
    type Error = Error;

    fn try_from(flags: c_int) -> Result<Self, Self::Error> {
        // Check if any unsupported flags is set.
        if flags & !FileStatusFlags::mask() != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "unsupported file status flags"));
        }

        let mut status_flags: FileStatusFlags = FileStatusFlags::empty();

        // Set append flag.
        if flags & FileStatusFlag::Append as c_int != 0 {
            status_flags = status_flags.set_append(true);
        }

        // Set non-blocking flag.
        if flags & FileStatusFlag::NonBlocking as c_int != 0 {
            status_flags = status_flags.set_non_blocking(true);
        }

        // Set synchronized I/O flag.
        if flags & FileStatusFlag::SynchronizedIo as c_int != 0 {
            status_flags = status_flags.set_synchronized_io(true);
        }

        Ok(status_flags)
    }
}
