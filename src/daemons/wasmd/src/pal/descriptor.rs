// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::posix::{
    ffi::c_int,
    sys::types::mode_t,
};

//==================================================================================================
// Types
//==================================================================================================

pub type RawDescriptor = posix::ffi::c_int;

//==================================================================================================
// Descriptor
//==================================================================================================

#[derive(Debug)]
pub struct Descriptor(RawDescriptor);

//==================================================================================================
// Implementations
//==================================================================================================

impl Descriptor {
    pub fn new(fd: RawDescriptor) -> Self {
        Self(fd)
    }

    pub fn rawfd(&self) -> RawDescriptor {
        self.0
    }

    pub fn openat(&self, path: &str, oflags: &OpenFlags, mode: &AccessMode) -> Result<Self, c_int> {
        let fd: RawDescriptor = posix::fcntl::openat(self.0, path, oflags.into(), mode.0);

        if fd < 0 {
            return Err(unsafe { posix::errno::errno });
        } else {
            return Ok(Self(fd));
        }
    }
}

//==================================================================================================
// OpenFlags
//==================================================================================================

pub enum ExclusiveOpenFlags {
    ExecuteOnly,
    ReadOnly,
    ReadWrite,
    SearchOnly,
    WriteOnly,
}

#[derive(Default)]
pub struct NonExclusiveOpenFlags {
    pub append: bool,
    pub close_on_exec: bool,
    pub close_on_fork: bool,
    pub create: bool,
    pub directory: bool,
    pub dsync: bool,
    pub exclusive: bool,
    pub no_controlling_terminal: bool,
    pub no_follow: bool,
    pub non_block: bool,
    pub rsync: bool,
    pub sync: bool,
    pub truncate: bool,
    pub initialize_tty: bool,
}

pub struct OpenFlags {
    pub exclusive: ExclusiveOpenFlags,
    pub non_exclusive: NonExclusiveOpenFlags,
}

impl OpenFlags {
    pub fn empty() -> Self {
        Self {
            exclusive: ExclusiveOpenFlags::ReadOnly,
            non_exclusive: NonExclusiveOpenFlags::default(),
        }
    }
}

impl From<&OpenFlags> for c_int {
    fn from(oflags: &OpenFlags) -> c_int {
        let mut flags: c_int = 0;

        match oflags.exclusive {
            ExclusiveOpenFlags::ExecuteOnly => flags |= posix::fcntl::O_EXEC,
            ExclusiveOpenFlags::ReadOnly => flags |= posix::fcntl::O_RDONLY,
            ExclusiveOpenFlags::ReadWrite => flags |= posix::fcntl::O_RDWR,
            ExclusiveOpenFlags::SearchOnly => flags |= posix::fcntl::O_SEARCH,
            ExclusiveOpenFlags::WriteOnly => flags |= posix::fcntl::O_WRONLY,
        }

        if oflags.non_exclusive.append {
            flags |= posix::fcntl::O_APPEND;
        }

        if oflags.non_exclusive.close_on_exec {
            unimplemented!("close-on-exec not supported yet");
        }

        if oflags.non_exclusive.close_on_fork {
            unimplemented!("close-on-fork not supported yet");
        }

        if oflags.non_exclusive.create {
            flags |= posix::fcntl::O_CREAT;
        }

        if oflags.non_exclusive.directory {
            flags |= posix::fcntl::O_DIRECTORY;
        }

        if oflags.non_exclusive.dsync {
            flags |= posix::fcntl::O_DSYNC;
        }

        if oflags.non_exclusive.exclusive {
            flags |= posix::fcntl::O_EXCL;
        }

        if oflags.non_exclusive.no_controlling_terminal {
            unimplemented!("no-controlling-terminal not supported yet");
        }

        if oflags.non_exclusive.no_follow {
            unimplemented!("no-follow not supported yet");
        }

        if oflags.non_exclusive.non_block {
            flags |= posix::fcntl::O_NONBLOCK;
        }

        if oflags.non_exclusive.rsync {
            flags |= posix::fcntl::O_RSYNC;
        }

        if oflags.non_exclusive.sync {
            flags |= posix::fcntl::O_SYNC;
        }

        if oflags.non_exclusive.truncate {
            flags |= posix::fcntl::O_TRUNC;
        }

        if oflags.non_exclusive.initialize_tty {
            unimplemented!("initialize-tty not supported yet");
        }

        flags
    }
}

#[derive(Default)]
pub struct AccessMode(mode_t);
