// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Statically allocated identifiers for synthetic filesystem metadata.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::{
    dev_t,
    ino_t,
};

//==================================================================================================
// Enumerations
//==================================================================================================

/// Device identifiers for synthetic filesystems.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilesystemDeviceId {
    /// The regular VFS file namespace.
    Vfs = 1,
    /// The unnamed-pipe filesystem.
    PipeFs = 2,
    /// The seeded console streams.
    Console = 3,
    /// The synthetic device filesystem.
    DevFs = 4,
    /// The host-backed filesystem.
    HostFs = 5,
    /// The socket filesystem.
    SockFs = 6,
}

/// Fixed inode identifiers in the regular VFS namespace.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VfsInodeId {
    /// Fallback inode used when the backing filesystem does not expose one.
    Fallback = 1,
}

/// Fixed inode identifiers in the hostfs namespace.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostFsInodeId {
    /// Fallback inode used because hostfsd does not expose host inode numbers.
    Fallback = 1,
}

/// Fixed inode identifiers in the devfs namespace.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevFsInodeId {
    /// The `/dev` directory.
    Directory = 1,
    /// The `/dev/null` device.
    Null = 2,
    /// The `/dev/tty` device.
    Tty = 3,
    /// The `/dev/console` device.
    Console = 4,
}

/// Fixed inode identifiers in the console-stream namespace.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleInodeId {
    /// Standard input.
    Stdin = 1,
    /// Standard output.
    Stdout = 2,
    /// Standard error.
    Stderr = 3,
}

/// Character-device identifiers in the devfs namespace.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevFsCharacterDeviceId {
    /// The null device.
    Null = 1,
    /// The controlling-terminal device.
    Tty = 2,
    /// The system console device.
    Console = 3,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl From<FilesystemDeviceId> for dev_t {
    fn from(value: FilesystemDeviceId) -> Self {
        value as Self
    }
}

impl From<VfsInodeId> for ino_t {
    fn from(value: VfsInodeId) -> Self {
        value as Self
    }
}

impl From<HostFsInodeId> for ino_t {
    fn from(value: HostFsInodeId) -> Self {
        value as Self
    }
}

impl From<DevFsInodeId> for ino_t {
    fn from(value: DevFsInodeId) -> Self {
        value as Self
    }
}

impl From<ConsoleInodeId> for ino_t {
    fn from(value: ConsoleInodeId) -> Self {
        value as Self
    }
}

impl From<DevFsCharacterDeviceId> for dev_t {
    fn from(value: DevFsCharacterDeviceId) -> Self {
        value as Self
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests;
