// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::fcntl::OpenFlags;
use ::sysapi::ffi::c_int;

//==================================================================================================
// File Control Request
//==================================================================================================

pub struct FileStatusFlags {
    flags: c_int,
}

impl FileStatusFlags {
    pub fn empty() -> Self {
        FileStatusFlags { flags: 0 }
    }

    pub fn set_append(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = OpenFlags::Append.into();
            self.flags |= flag;
        } else {
            let flag: c_int = OpenFlags::Append.into();
            self.flags &= !flag;
        }
        self
    }

    pub fn set_synchronized_writes(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = OpenFlags::Sync.into();
            self.flags |= flag;
        } else {
            let flag: c_int = OpenFlags::Sync.into();
            self.flags &= !flag;
        }
        self
    }

    pub fn set_non_blocking(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = OpenFlags::NonBlocking.into();
            self.flags |= flag;
        } else {
            let flag: c_int = OpenFlags::NonBlocking.into();
            self.flags &= !flag;
        }
        self
    }

    pub fn set_synchronized_reads(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = OpenFlags::Sync.into();
            self.flags |= flag;
        } else {
            let flag: c_int = OpenFlags::Sync.into();
            self.flags &= !flag;
        }
        self
    }
}

impl From<c_int> for FileStatusFlags {
    fn from(flags: c_int) -> Self {
        FileStatusFlags { flags }
    }
}

impl From<FileStatusFlags> for c_int {
    fn from(flag: FileStatusFlags) -> Self {
        flag.flags
    }
}
