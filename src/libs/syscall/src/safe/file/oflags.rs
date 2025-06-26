// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::safe::OpenFlags;
use ::sysapi::ffi::c_int;

//==================================================================================================
// OpenOptions
//==================================================================================================

///
/// # Description
///
/// A structure that represents the options for opening a regular file.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RegularFileOpenFlags {
    flags: OpenFlags,
}

impl RegularFileOpenFlags {
    pub fn read_only() -> Self {
        Self {
            flags: OpenFlags::read_only(),
        }
    }

    pub fn write_only() -> Self {
        Self {
            flags: OpenFlags::write_only(),
        }
    }

    pub fn read_write() -> Self {
        Self {
            flags: OpenFlags::read_write(),
        }
    }

    pub fn execute_only() -> Self {
        Self {
            flags: OpenFlags::execute_only(),
        }
    }

    pub fn set_create(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_create(enable);
        self
    }

    pub fn set_truncate(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_truncate(enable);
        self
    }

    pub fn set_exclusive(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_exclusive(enable);
        self
    }

    pub fn set_no_follow(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_no_follow(enable);
        self
    }

    pub fn set_close_on_exec(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_close_on_exec(enable);
        self
    }

    pub fn set_close_on_fork(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_close_on_fork(enable);
        self
    }

    pub fn set_append(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_append(enable);
        self
    }

    pub fn set_non_blocking(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_non_blocking(enable);
        self
    }

    pub fn set_synchronized_io(mut self, enable: bool) -> Self {
        self.flags = self.flags.set_synchronized_io(enable);
        self
    }
}

impl From<RegularFileOpenFlags> for c_int {
    fn from(flags: RegularFileOpenFlags) -> Self {
        flags.flags.into()
    }
}

impl From<&RegularFileOpenFlags> for c_int {
    fn from(flags: &RegularFileOpenFlags) -> Self {
        flags.flags.into()
    }
}
