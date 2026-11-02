// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Synthetic `/dev` namespace.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    filesystem::{
        DirEntry,
        Stat,
    },
    identifiers::{
        DevFsCharacterDeviceId,
        DevFsInodeId,
        FilesystemDeviceId,
    },
    mount::normalize_anchored,
    path::AnchoredPath,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::fat32::{
    Fat32Error,
    FAT_EPOCH_SECS,
};
use ::sys::pm::{
    GroupIdentifier,
    UserIdentifier,
};
use ::sysapi::{
    sys_stat::{
        file_mode,
        file_type,
        stat as PosixStat,
    },
    sys_types::{
        gid_t,
        uid_t,
    },
    time::timespec,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Name of the devfs root directory.
const DIRECTORY_NAME: &str = "dev";
/// Absolute path of the devfs root directory.
const DIRECTORY_PATH: &str = "/dev";
/// Absolute prefix for devfs entries.
const DIRECTORY_PREFIX: &str = "/dev/";

/// Preferred I/O block size reported for devfs entries.
const STAT_BLOCK_SIZE: i64 = ::arch::mem::PAGE_SIZE as i64;

/// Stable timestamp used until VFS defines backend-neutral synthetic timestamps.
const STAT_TIMESTAMP_SECS: i64 = FAT_EPOCH_SECS;

/// Name of the null device.
const NULL_NAME: &str = "null";
/// Absolute path of the null device.
const NULL_PATH: &str = "/dev/null";

/// Name of the controlling-terminal device.
const TTY_NAME: &str = "tty";
/// Absolute path of the controlling-terminal device.
const TTY_PATH: &str = "/dev/tty";

/// Name of the console device.
const CONSOLE_NAME: &str = "console";
/// Absolute path of the console device.
const CONSOLE_PATH: &str = "/dev/console";

//==================================================================================================
// Structures
//==================================================================================================

/// Metadata for a synthetic device-namespace entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeviceMetadata {
    /// Stable inode identifier.
    inode: DevFsInodeId,
    /// Device identifier for a character-special entry.
    special_device: Option<DevFsCharacterDeviceId>,
    /// Whether this entry is a directory.
    is_directory: bool,
}

//==================================================================================================
// Enumerations
//==================================================================================================

/// A path resolved within the synthetic device namespace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DevicePath {
    /// The synthetic `/dev` directory.
    Directory,
    /// The null device.
    Null,
    /// The calling process's controlling terminal.
    Tty,
    /// The system console.
    Console,
    /// An unknown entry below `/dev`.
    Missing,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl DevicePath {
    /// Returns metadata for an existing namespace entry.
    fn metadata(self) -> Option<DeviceMetadata> {
        match self {
            DevicePath::Directory => Some(DeviceMetadata {
                inode: DevFsInodeId::Directory,
                special_device: None,
                is_directory: true,
            }),
            DevicePath::Null => Some(DeviceMetadata {
                inode: DevFsInodeId::Null,
                special_device: Some(DevFsCharacterDeviceId::Null),
                is_directory: false,
            }),
            DevicePath::Tty => Some(DeviceMetadata {
                inode: DevFsInodeId::Tty,
                special_device: Some(DevFsCharacterDeviceId::Tty),
                is_directory: false,
            }),
            DevicePath::Console => Some(DeviceMetadata {
                inode: DevFsInodeId::Console,
                special_device: Some(DevFsCharacterDeviceId::Console),
                is_directory: false,
            }),
            DevicePath::Missing => None,
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Resolves raw path provenance in the synthetic device namespace.
pub(crate) fn resolve_anchored(path: AnchoredPath) -> Result<Option<DevicePath>, Fat32Error> {
    let absolute: String = path.into_absolute()?;
    resolve_absolute(&absolute)
}

/// Returns whether an anchored path is owned by the synthetic device namespace.
pub(crate) fn owns(path: AnchoredPath) -> Result<bool, Fat32Error> {
    let absolute: String = path.into_absolute()?;
    match resolve_absolute(&absolute) {
        Ok(path) => Ok(path.is_some()),
        Err(Fat32Error::NotFound | Fat32Error::NotADirectory) => Ok(true),
        Err(error) => Err(error),
    }
}

/// Validates a joined path and classifies its normalized namespace path.
pub(crate) fn resolve_joined(
    joined: &str,
    normalized: &str,
) -> Result<Option<DevicePath>, Fat32Error> {
    validate_components(joined)?;
    Ok(resolve_normalized(normalized))
}

/// Synthesizes backend-neutral metadata for an existing devfs path.
pub(crate) fn stat(path: DevicePath) -> Result<Stat, Fat32Error> {
    let metadata: DeviceMetadata = path.metadata().ok_or(Fat32Error::NotFound)?;
    Ok(Stat::new(
        0,
        metadata.is_directory,
        STAT_TIMESTAMP_SECS,
        STAT_TIMESTAMP_SECS,
        STAT_TIMESTAMP_SECS,
    ))
}

/// Synthesizes POSIX metadata for an existing devfs path.
pub(crate) fn posix_stat(path: DevicePath) -> Result<PosixStat, Fat32Error> {
    let metadata: DeviceMetadata = path.metadata().ok_or(Fat32Error::NotFound)?;
    Ok(build_posix_stat(metadata))
}

/// Synthesizes POSIX metadata for `/dev/null`.
pub(crate) fn null_posix_stat() -> PosixStat {
    build_posix_stat(
        DevicePath::Null
            .metadata()
            .expect("null device has metadata"),
    )
}

/// Synthesizes POSIX metadata for `/dev/tty`.
pub(crate) fn tty_posix_stat() -> PosixStat {
    build_posix_stat(DevicePath::Tty.metadata().expect("tty device has metadata"))
}

/// Synthesizes POSIX metadata for `/dev/console`.
pub(crate) fn console_posix_stat() -> PosixStat {
    build_posix_stat(
        DevicePath::Console
            .metadata()
            .expect("console device has metadata"),
    )
}

/// Builds POSIX metadata for a devfs entry.
fn build_posix_stat(metadata: DeviceMetadata) -> PosixStat {
    let timestamp: timespec = timespec {
        tv_sec: STAT_TIMESTAMP_SECS,
        tv_nsec: 0,
    };
    PosixStat {
        st_dev: FilesystemDeviceId::DevFs.into(),
        st_ino: metadata.inode.into(),
        st_mode: if metadata.is_directory {
            file_type::S_IFDIR | file_mode::S_IRWXU
        } else {
            file_type::S_IFCHR | file_mode::S_IRUSR | file_mode::S_IWUSR
        },
        st_nlink: if metadata.is_directory { 2 } else { 1 },
        st_uid: UserIdentifier::ROOT.as_usize() as uid_t,
        st_gid: GroupIdentifier::ROOT.as_usize() as gid_t,
        st_rdev: metadata.special_device.map_or(0, Into::into),
        st_size: 0,
        st_blksize: STAT_BLOCK_SIZE,
        st_blocks: 0,
        st_atim: timestamp,
        st_mtim: timestamp,
        st_ctim: timestamp,
    }
}

/// Returns the `/dev` entry injected into the VFS root directory.
pub(crate) fn directory_entry() -> DirEntry {
    DirEntry::new(String::from(DIRECTORY_NAME), DevFsInodeId::Directory.into(), true, 0)
}

/// Reads a directory owned by devfs.
pub(crate) fn read_dir(path: DevicePath) -> Result<Vec<DirEntry>, Fat32Error> {
    match path {
        DevicePath::Directory => Ok(alloc::vec![
            DirEntry::new_character_device(String::from(NULL_NAME), DevFsInodeId::Null.into()),
            DirEntry::new_character_device(String::from(TTY_NAME), DevFsInodeId::Tty.into()),
            DirEntry::new_character_device(
                String::from(CONSOLE_NAME),
                DevFsInodeId::Console.into(),
            ),
        ]),
        DevicePath::Null | DevicePath::Tty | DevicePath::Console => Err(Fat32Error::NotADirectory),
        DevicePath::Missing => Err(Fat32Error::NotFound),
    }
}

/// Validates and classifies an absolute namespace path.
fn resolve_absolute(path: &str) -> Result<Option<DevicePath>, Fat32Error> {
    let normalized: String = normalize_anchored(path);
    resolve_joined(path, &normalized)
}

/// Rejects traversal through an unknown namespace entry.
fn validate_components(path: &str) -> Result<(), Fat32Error> {
    let mut components: Vec<&str> = Vec::new();
    for component in path.split('/').filter(|component| !component.is_empty()) {
        if components.len() >= 2 && components[0] == DIRECTORY_NAME {
            return match components[1] {
                NULL_NAME | TTY_NAME | CONSOLE_NAME => Err(Fat32Error::NotADirectory),
                _ => Err(Fat32Error::NotFound),
            };
        }
        match component {
            "." => {},
            ".." => {
                components.pop();
            },
            component => components.push(component),
        }
    }
    Ok(())
}

/// Classifies a normalized, absolute path.
fn resolve_normalized(path: &str) -> Option<DevicePath> {
    match path {
        DIRECTORY_PATH => Some(DevicePath::Directory),
        NULL_PATH => Some(DevicePath::Null),
        TTY_PATH => Some(DevicePath::Tty),
        CONSOLE_PATH => Some(DevicePath::Console),
        path if path.starts_with(DIRECTORY_PREFIX) => Some(DevicePath::Missing),
        _ => None,
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
