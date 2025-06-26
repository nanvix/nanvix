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
    fcntl::file_creation_flags::{
        O_CLOEXEC,
        O_CLOFORK,
        O_CREAT,
        O_DIRECTORY,
        O_EXCL,
        O_NOCTTY,
        O_NOFOLLOW,
        O_TRUNC,
    },
    ffi::c_int,
};

//==================================================================================================
// FileCreationFlag
//==================================================================================================

/// A file creation flag to be used with `open()`, `openat()`, and `fcntl()` system calls.
#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FileCreationFlag {
    /// Create file if it does not exist.
    Create = O_CREAT,
    /// Truncate file to size zero.
    Truncate = O_TRUNC,
    /// Fail if not a new file.
    Exclusive = O_EXCL,
    /// Do not assign controlling terminal.
    NoControllingTerminal = O_NOCTTY,
    /// Do not follow symbolic links.
    NoFollow = O_NOFOLLOW,
    /// Fail if path resolves to a non-directory file.
    Directory = O_DIRECTORY,
    /// Close-on-exec.
    CloseOnExec = O_CLOEXEC,
    /// Close-on-fork.
    CloseOnFork = O_CLOFORK,
}

impl From<FileCreationFlag> for c_int {
    fn from(file_creation_flags: FileCreationFlag) -> Self {
        file_creation_flags as c_int
    }
}

//==================================================================================================
// FileCreationFlags
//==================================================================================================

///
/// # Description
///
/// File creation flags to be used with `open()`, `openat()`, and `fcntl()` system calls.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileCreationFlags {
    flags: c_int,
}

impl FileCreationFlags {
    /// Creates a new `FileCreationFlags` instance with no flags set.
    const fn empty() -> Self {
        FileCreationFlags { flags: 0 }
    }

    ///
    /// # Description
    ///
    /// Returns a bitmask of all file creation flags.
    ///
    /// # Returns
    ///
    /// A bitmask of all file creation flags.
    ///
    pub const fn mask() -> c_int {
        (FileCreationFlag::Create as c_int)
            | (FileCreationFlag::Truncate as c_int)
            | (FileCreationFlag::Exclusive as c_int)
            | (FileCreationFlag::NoControllingTerminal as c_int)
            | (FileCreationFlag::NoFollow as c_int)
            | (FileCreationFlag::Directory as c_int)
            | (FileCreationFlag::CloseOnExec as c_int)
            | (FileCreationFlag::CloseOnFork as c_int)
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileCreationFlags` instance with the create flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the create flag is set; if `false`, the create flag is unset.
    ///
    /// # Returns
    ///
    /// A new `FileCreationFlags` instance with the create flag set or unset.
    ///
    pub fn set_create(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileCreationFlag::Create.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileCreationFlag::Create.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileCreationFlags` instance with the truncate flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the truncate flag is set; if `false`, the truncate flag is unset.
    ///
    /// # Returns
    ///
    /// A new `FileCreationFlags` instance with the truncate flag set or unset.
    ///
    pub fn set_truncate(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileCreationFlag::Truncate.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileCreationFlag::Truncate.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileCreationFlags` instance with the exclusive flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the exclusive flag is set; if `false`, the exclusive flag is unset.
    ///
    /// # Returns
    ///
    /// A new `FileCreationFlags` instance with the exclusive flag set or unset.
    ///
    pub fn set_exclusive(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileCreationFlag::Exclusive.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileCreationFlag::Exclusive.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileCreationFlags` instance with the no controlling terminal flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the no controlling terminal flag is set; if `false`, the no
    ///   controlling terminal flag is unset.
    ///
    /// # Returns
    ///
    /// A new `FileCreationFlags` instance with the no controlling terminal flag set or unset.
    ///
    pub fn set_no_controlling_terminal(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileCreationFlag::NoControllingTerminal.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileCreationFlag::NoControllingTerminal.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileCreationFlags` instance with the no follow flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the no follow flag is set; if `false`, the no follow flag is unset.
    ///
    /// # Returns
    ///
    /// A new `FileCreationFlags` instance with the no follow flag set or unset.
    ///
    pub fn set_no_follow(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileCreationFlag::NoFollow.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileCreationFlag::NoFollow.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileCreationFlags` instance with the directory flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the directory flag is set; if `false`, the directory flag is unset.
    ///
    /// # Returns
    ///
    /// A new `FileCreationFlags` instance with the directory flag set or unset.
    ///
    pub fn set_directory(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileCreationFlag::Directory.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileCreationFlag::Directory.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileCreationFlags` instance with the close-on-exec flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the close-on-exec flag is set; if `false`, the close-on-exec flag
    ///   is unset.
    ///
    /// # Returns
    ///
    /// A new `FileCreationFlags` instance with the close-on-exec flag set or unset.
    ///
    pub fn set_close_on_exec(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileCreationFlag::CloseOnExec.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileCreationFlag::CloseOnExec.into();
            self.flags &= !flag;
        }
        self
    }

    ///
    /// # Description
    ///
    /// Creates a new `FileCreationFlags` instance with the close-on-fork flag set/unset.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the close-on-fork flag is set; if `false`, the close-on-fork flag
    ///   is unset.
    ///
    /// # Returns
    ///
    /// A new `FileCreationFlags` instance with the close-on-fork flag set or unset.
    ///
    pub fn set_close_on_fork(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = FileCreationFlag::CloseOnFork.into();
            self.flags |= flag;
        } else {
            let flag: c_int = FileCreationFlag::CloseOnFork.into();
            self.flags &= !flag;
        }
        self
    }
}

impl Default for FileCreationFlags {
    fn default() -> Self {
        FileCreationFlags::empty()
    }
}

impl From<FileCreationFlags> for c_int {
    fn from(flag: FileCreationFlags) -> Self {
        flag.flags
    }
}

impl From<&FileCreationFlags> for c_int {
    fn from(flag: &FileCreationFlags) -> Self {
        flag.flags
    }
}

impl TryFrom<c_int> for FileCreationFlags {
    type Error = Error;

    fn try_from(flags: c_int) -> Result<Self, Self::Error> {
        // Check if any unsupported flags is set.
        if flags & !FileCreationFlags::mask() != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "unsupported file creation flags"));
        }

        let mut creation_flags: FileCreationFlags = FileCreationFlags::empty();

        // Set create flag.
        if flags & FileCreationFlag::Create as c_int != 0 {
            creation_flags = creation_flags.set_create(true);
        }

        // Set truncate flag.
        if flags & FileCreationFlag::Truncate as c_int != 0 {
            creation_flags = creation_flags.set_truncate(true);
        }

        // Set exclusive flag.
        if flags & FileCreationFlag::Exclusive as c_int != 0 {
            creation_flags = creation_flags.set_exclusive(true);
        }

        // Set no controlling terminal flag.
        if flags & FileCreationFlag::NoControllingTerminal as c_int != 0 {
            creation_flags = creation_flags.set_no_controlling_terminal(true);
        }

        // Set no follow flag.
        if flags & FileCreationFlag::NoFollow as c_int != 0 {
            creation_flags = creation_flags.set_no_follow(true);
        }

        // Set directory flag.
        if flags & FileCreationFlag::Directory as c_int != 0 {
            creation_flags = creation_flags.set_directory(true);
        }

        // Set close-on-exec flag.
        if flags & FileCreationFlag::CloseOnExec as c_int != 0 {
            creation_flags = creation_flags.set_close_on_exec(true);
        }

        // Set close-on-fork flag.
        if flags & FileCreationFlag::CloseOnFork as c_int != 0 {
            creation_flags = creation_flags.set_close_on_fork(true);
        }

        Ok(creation_flags)
    }
}
