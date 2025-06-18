// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ops::{
    BitAnd,
    BitOr,
    BitOrAssign,
};
use ::sysapi::fcntl::open_flags::{
    O_APPEND,
    O_CLOEXEC,
    O_CREAT,
    O_DIRECTORY,
    O_EXCL,
    O_NONBLOCK,
    O_RDONLY,
    O_RDWR,
    O_SYNC,
    O_TRUNC,
    O_WRONLY,
};

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        pub mod syscall;
        pub use self::syscall::{
            creat,
            openat,
            unlinkat,
            rename,
            renameat,
            posix_fallocate,
            posix_fadvise,
            fcntl,
            open,
        };
    }
}

//==================================================================================================

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDescriptorFlags {
    /// Close-on-exec flag.
    CloseOnExec = O_CLOEXEC,
    /// Non-blocking mode.
    NonBlocking = O_NONBLOCK,
}

impl From<FileDescriptorFlags> for i32 {
    fn from(flag: FileDescriptorFlags) -> Self {
        flag as i32
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenFlags {
    /// Set read-only access.
    Readonly = O_RDONLY,
    /// Set write-only access.
    WriteOnly = O_WRONLY,
    /// Set read-write access.
    ReadWrite = O_RDWR,
    /// Set append mode.
    Append = O_APPEND,
    /// Create file if it does not exist.
    Create = O_CREAT,
    /// Truncate file to size zero.
    Truncate = O_TRUNC,
    /// Fail if not a new file.
    Exclusive = O_EXCL,
    /// Write I/O operations on the file descriptor will complete as defined by synchronized I/O data integrity completion.
    Sync = O_SYNC,
    /// Non-blocking mode.
    NonBlocking = O_NONBLOCK,
    /// Fail if path resolves to a non-directory file.
    Directory = O_DIRECTORY,
}

impl BitOr for OpenFlags {
    type Output = i32;

    fn bitor(self, rhs: OpenFlags) -> Self::Output {
        (self as i32) | (rhs as i32)
    }
}

impl BitOr<i32> for OpenFlags {
    type Output = i32;

    fn bitor(self, rhs: i32) -> Self::Output {
        (self as i32) | rhs
    }
}

impl BitOr<OpenFlags> for i32 {
    type Output = i32;

    fn bitor(self, rhs: OpenFlags) -> Self::Output {
        self | (rhs as i32)
    }
}

impl BitOrAssign<OpenFlags> for i32 {
    fn bitor_assign(&mut self, rhs: OpenFlags) {
        *self |= rhs as i32;
    }
}

impl BitAnd for OpenFlags {
    type Output = i32;

    fn bitand(self, rhs: OpenFlags) -> Self::Output {
        (self as i32) & (rhs as i32)
    }
}

impl BitAnd<i32> for OpenFlags {
    type Output = i32;

    fn bitand(self, rhs: i32) -> Self::Output {
        (self as i32) & rhs
    }
}

impl BitAnd<OpenFlags> for i32 {
    type Output = i32;

    fn bitand(self, rhs: OpenFlags) -> Self::Output {
        self & (rhs as i32)
    }
}

impl BitAnd<&OpenFlags> for i32 {
    type Output = i32;

    fn bitand(self, rhs: &OpenFlags) -> Self::Output {
        self & (*rhs as i32)
    }
}

impl From<OpenFlags> for i32 {
    fn from(flag: OpenFlags) -> Self {
        flag as i32
    }
}

impl From<&OpenFlags> for i32 {
    fn from(flag: &OpenFlags) -> Self {
        *flag as i32
    }
}
