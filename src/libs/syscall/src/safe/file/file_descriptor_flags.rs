// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl,
    ffi::c_int,
};

//==================================================================================================
// File Control Request
//==================================================================================================

pub struct FileDescriptorFlags {
    flags: c_int,
}

impl FileDescriptorFlags {
    pub fn empty() -> Self {
        FileDescriptorFlags { flags: 0 }
    }

    pub fn set_close_on_exec(mut self, enable: bool) -> Self {
        if enable {
            let flag: c_int = fcntl::FileDescriptorFlags::O_CLOEXEC.into();
            self.flags |= flag;
        } else {
            let flag: c_int = fcntl::FileDescriptorFlags::O_CLOEXEC.into();
            self.flags &= !flag;
        }
        self
    }
}

impl From<c_int> for FileDescriptorFlags {
    fn from(flags: c_int) -> Self {
        FileDescriptorFlags { flags }
    }
}

impl From<FileDescriptorFlags> for c_int {
    fn from(flag: FileDescriptorFlags) -> Self {
        flag.flags
    }
}
