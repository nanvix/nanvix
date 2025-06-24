// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::{
    FileAccessModeFlags,
    FileCreationFlags,
    FileStatusFlags,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Static Asserts
//==================================================================================================

// Ensure open flags do not overlap.
::static_assert::assert_eq!((FileAccessModeFlags::mask() & FileCreationFlags::mask()) == 0);
::static_assert::assert_eq!((FileAccessModeFlags::mask() & FileStatusFlags::mask()) == 0);
::static_assert::assert_eq!((FileCreationFlags::mask() & FileStatusFlags::mask()) == 0);

//==================================================================================================
//  OpenFlags
//==================================================================================================

///
/// # Description
///
///  Open flags to be used with `open()` and  `openat()` system calls.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct OpenFlags {
    /// Access mode flags.
    access_mode_flags: FileAccessModeFlags,
    /// Status flags.
    status_flags: FileStatusFlags,
    /// Creation flags.
    creation_flags: FileCreationFlags,
}

impl OpenFlags {
    ///
    /// # Description
    ///
    /// Creates a new `OpenFlags` instance with read-only access mode.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance configured for read-only access with default status and creation flags.
    ///
    pub fn read_only() -> Self {
        Self {
            access_mode_flags: FileAccessModeFlags::readonly(),
            status_flags: FileStatusFlags::default(),
            creation_flags: FileCreationFlags::default(),
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `OpenFlags` instance with write-only access mode.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance configured for write-only access with default status and creation flags.
    ///
    pub fn write_only() -> Self {
        Self {
            access_mode_flags: FileAccessModeFlags::write_only(),
            status_flags: FileStatusFlags::default(),
            creation_flags: FileCreationFlags::default(),
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `OpenFlags` instance with read-write access mode.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance configured for read-write access with default status and creation flags.
    ///
    pub fn read_write() -> Self {
        Self {
            access_mode_flags: FileAccessModeFlags::read_write(),
            status_flags: FileStatusFlags::default(),
            creation_flags: FileCreationFlags::default(),
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new `OpenFlags` instance with execute-only access mode.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance configured for execute-only access with default status and creation flags.
    ///
    pub fn execute_only() -> Self {
        Self {
            access_mode_flags: FileAccessModeFlags::execute_only(),
            status_flags: FileStatusFlags::default(),
            creation_flags: FileCreationFlags::default(),
        }
    }

    ///
    /// # Description
    ///
    ///  Creates a new `OpenFlags` instance with search-only access mode.
    ///
    /// # Returns
    ///
    ///  A new `OpenFlags` instance configured for search-only access with default status and creation flags.
    ///
    pub fn search_only() -> Self {
        Self {
            access_mode_flags: FileAccessModeFlags::search_only(),
            status_flags: FileStatusFlags::default(),
            creation_flags: FileCreationFlags::default(),
        }
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the create flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the create flag is set; if `false`, the create flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the create flag updated.
    ///
    pub fn set_create(mut self, enable: bool) -> Self {
        self.creation_flags = self.creation_flags.set_create(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the truncate flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the truncate flag is set; if `false`, the truncate flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the truncate flag updated.
    ///
    pub fn set_truncate(mut self, enable: bool) -> Self {
        self.creation_flags = self.creation_flags.set_truncate(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the exclusive flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the exclusive flag is set; if `false`, the exclusive flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the exclusive flag updated.
    ///
    pub fn set_exclusive(mut self, enable: bool) -> Self {
        self.creation_flags = self.creation_flags.set_exclusive(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the no controlling terminal flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the no controlling terminal flag is set; if `false`, the no
    ///   controlling terminal flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the no controlling terminal flag updated.
    ///
    pub fn set_no_controlling_terminal(mut self, enable: bool) -> Self {
        self.creation_flags = self.creation_flags.set_no_controlling_terminal(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the no follow flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the no follow flag is set; if `false`, the no follow flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the no follow flag updated.
    ///
    pub fn set_no_follow(mut self, enable: bool) -> Self {
        self.creation_flags = self.creation_flags.set_no_follow(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the directory flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the directory flag is set; if `false`, the directory flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the directory flag updated.
    ///
    pub fn set_directory(mut self, enable: bool) -> Self {
        self.creation_flags = self.creation_flags.set_directory(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the close-on-exec flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the close-on-exec flag is set; if `false`, the close-on-exec flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the close-on-exec flag updated.
    ///
    pub fn set_close_on_exec(mut self, enable: bool) -> Self {
        self.creation_flags = self.creation_flags.set_close_on_exec(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the close-on-fork flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the close-on-fork flag is set; if `false`, the close-on-fork flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the close-on-fork flag updated.
    ///
    pub fn set_close_on_fork(mut self, enable: bool) -> Self {
        self.creation_flags = self.creation_flags.set_close_on_fork(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the append flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the append flag is set; if `false`, the append flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the append flag updated.
    ///
    pub fn set_append(mut self, enable: bool) -> Self {
        self.status_flags = self.status_flags.set_append(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the non-blocking flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the non-blocking flag is set; if `false`, the non-blocking flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the non-blocking flag updated.
    ///
    pub fn set_non_blocking(mut self, enable: bool) -> Self {
        self.status_flags = self.status_flags.set_non_blocking(enable);
        self
    }

    ///
    /// # Description
    ///
    /// Sets or unsets the synchronized I/O flag on this `OpenFlags` instance.
    ///
    /// # Arguments
    ///
    /// - `enable`: If `true`, the synchronized I/O flag is set; if `false`, the synchronized I/O flag is unset.
    ///
    /// # Returns
    ///
    /// A new `OpenFlags` instance with the synchronized I/O flag updated.
    ///
    pub fn set_synchronized_io(mut self, enable: bool) -> Self {
        self.status_flags = self.status_flags.set_synchronized_io(enable);
        self
    }
}

impl From<OpenFlags> for c_int {
    fn from(flags: OpenFlags) -> Self {
        let access_mode_flags: c_int = flags.access_mode_flags.into();
        let creation_flags: c_int = flags.creation_flags.into();
        let status_flags: c_int = flags.status_flags.into();
        access_mode_flags | creation_flags | status_flags
    }
}

impl From<&OpenFlags> for c_int {
    fn from(flags: &OpenFlags) -> Self {
        let access_mode_flags: c_int = flags.access_mode_flags.into();
        let creation_flags: c_int = flags.creation_flags.into();
        let status_flags: c_int = flags.status_flags.into();
        access_mode_flags | creation_flags | status_flags
    }
}

impl From<&mut OpenFlags> for c_int {
    fn from(flags: &mut OpenFlags) -> Self {
        let access_mode_flags: c_int = flags.access_mode_flags.into();
        let creation_flags: c_int = flags.creation_flags.into();
        let status_flags: c_int = flags.status_flags.into();
        access_mode_flags | creation_flags | status_flags
    }
}

impl TryFrom<c_int> for OpenFlags {
    type Error = Error;

    fn try_from(flags: c_int) -> Result<Self, Self::Error> {
        let access_mode_mask: c_int = FileAccessModeFlags::mask();
        let creation_mask: c_int = FileCreationFlags::mask();
        let status_mask: c_int = FileStatusFlags::mask();

        // Check if any unsupported flags are set.
        if flags & !(access_mode_mask | creation_mask | status_mask) != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "Unsupported open flags "));
        }

        // Attempt to extract access mode flags.
        let access_mode_flags: FileAccessModeFlags =
            FileAccessModeFlags::try_from(flags & access_mode_mask)?;

        // Attempt to extract creation flags.
        let creation_flags: FileCreationFlags = FileCreationFlags::try_from(flags & creation_mask)?;

        // Attempt to extract status flags.
        let status_flags: FileStatusFlags = FileStatusFlags::try_from(flags & status_mask)?;

        Ok(OpenFlags {
            access_mode_flags,
            status_flags,
            creation_flags,
        })
    }
}
