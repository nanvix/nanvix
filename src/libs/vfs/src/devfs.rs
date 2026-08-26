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
    mount::{
        anchor_path,
        normalize_anchored,
    },
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

/// Synthetic device identifier for devfs.
///
/// IDs 1 through 3 identify the VFS file namespace, pipefs, and console.
const DEVICE_NAMESPACE_ID: u64 = 4;

/// Name of the devfs root directory.
const DIRECTORY_NAME: &str = "dev";
/// Absolute path of the devfs root directory.
const DIRECTORY_PATH: &str = "/dev";
/// Absolute prefix for devfs entries.
const DIRECTORY_PREFIX: &str = "/dev/";

/// Stable inode identifier for `/dev`.
///
/// Inode zero is reserved, so `/dev` uses the first available identifier.
const DIRECTORY_INODE: u64 = 1;

/// Preferred I/O block size reported for devfs entries.
const STAT_BLOCK_SIZE: i64 = ::arch::mem::PAGE_SIZE as i64;

/// Stable timestamp used until VFS defines backend-neutral synthetic timestamps.
const STAT_TIMESTAMP_SECS: i64 = FAT_EPOCH_SECS;

/// Name of the null device.
const NULL_NAME: &str = "null";
/// Absolute path of the null device.
const NULL_PATH: &str = "/dev/null";

/// Stable inode identifier for `/dev/null`.
const NULL_INODE: u64 = 2;

/// Character-device identifier reported for `/dev/null`.
const NULL_SPECIAL_DEVICE_ID: u64 = 1;

//==================================================================================================
// Structures
//==================================================================================================

/// Metadata for a synthetic device-namespace entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeviceMetadata {
    /// Synthetic device identifier.
    device: u64,
    /// Stable inode identifier.
    inode: u64,
    /// Device identifier for a character-special entry.
    special_device: u64,
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
                device: DEVICE_NAMESPACE_ID,
                inode: DIRECTORY_INODE,
                special_device: 0,
                is_directory: true,
            }),
            DevicePath::Null => Some(DeviceMetadata {
                device: DEVICE_NAMESPACE_ID,
                inode: NULL_INODE,
                special_device: NULL_SPECIAL_DEVICE_ID,
                is_directory: false,
            }),
            DevicePath::Missing => None,
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Resolves a path in the synthetic device namespace.
pub(crate) fn resolve(cwd: &str, path: &str) -> Result<Option<DevicePath>, Fat32Error> {
    let anchored: String = anchor_path(path, cwd)?;
    validate_components(&anchored)?;
    let normalized: String = normalize_anchored(&anchored);
    Ok(resolve_normalized(&normalized))
}

/// Returns whether routing for a path belongs to the synthetic device namespace.
pub(crate) fn owns(cwd: &str, path: &str) -> Result<bool, Fat32Error> {
    let anchored: String = anchor_path(path, cwd)?;
    if validate_components(&anchored).is_err() {
        return Ok(true);
    }
    let normalized: String = normalize_anchored(&anchored);
    Ok(resolve_normalized(&normalized).is_some())
}

/// Resolves metadata for an existing device-namespace path.
fn metadata(cwd: &str, path: &str) -> Result<Option<DeviceMetadata>, Fat32Error> {
    let anchored: String = anchor_path(path, cwd)?;
    validate_components(&anchored)?;
    let normalized: String = normalize_anchored(&anchored);
    let Some(device_path) = resolve_normalized(&normalized) else {
        return Ok(None);
    };
    let metadata: DeviceMetadata = device_path.metadata().ok_or(Fat32Error::NotFound)?;
    if path.ends_with('/') && !metadata.is_directory {
        return Err(Fat32Error::NotADirectory);
    }
    Ok(Some(metadata))
}

/// Synthesizes backend-neutral metadata for an existing devfs path.
pub(crate) fn stat(cwd: &str, path: &str) -> Result<Option<Stat>, Fat32Error> {
    let Some(metadata) = metadata(cwd, path)? else {
        return Ok(None);
    };
    Ok(Some(Stat::new(
        0,
        metadata.is_directory,
        STAT_TIMESTAMP_SECS,
        STAT_TIMESTAMP_SECS,
        STAT_TIMESTAMP_SECS,
    )))
}

/// Synthesizes POSIX metadata for an existing devfs path.
pub(crate) fn posix_stat(cwd: &str, path: &str) -> Result<Option<PosixStat>, Fat32Error> {
    Ok(metadata(cwd, path)?.map(build_posix_stat))
}

/// Synthesizes POSIX metadata for `/dev/null`.
pub(crate) fn null_posix_stat() -> PosixStat {
    build_posix_stat(DevicePath::Null.metadata().expect("null device has metadata"))
}

/// Builds POSIX metadata for a devfs entry.
fn build_posix_stat(metadata: DeviceMetadata) -> PosixStat {
    let timestamp: timespec = timespec {
        tv_sec: STAT_TIMESTAMP_SECS,
        tv_nsec: 0,
    };
    PosixStat {
        st_dev: metadata.device,
        st_ino: metadata.inode,
        st_mode: if metadata.is_directory {
            file_type::S_IFDIR | file_mode::S_IRWXU
        } else {
            file_type::S_IFCHR | file_mode::S_IRUSR | file_mode::S_IWUSR
        },
        st_nlink: if metadata.is_directory { 2 } else { 1 },
        st_uid: UserIdentifier::ROOT.as_usize() as uid_t,
        st_gid: GroupIdentifier::ROOT.as_usize() as gid_t,
        st_rdev: metadata.special_device,
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
    DirEntry::new(String::from(DIRECTORY_NAME), DIRECTORY_INODE, true, 0)
}

/// Reads a directory owned by devfs.
pub(crate) fn read_dir(cwd: &str, path: &str) -> Result<Option<Vec<DirEntry>>, Fat32Error> {
    match resolve(cwd, path)? {
        Some(DevicePath::Directory) => Ok(Some(alloc::vec![DirEntry::new_character_device(
            String::from(NULL_NAME),
            NULL_INODE,
        )])),
        Some(DevicePath::Null) => Err(Fat32Error::NotADirectory),
        Some(DevicePath::Missing) => Err(Fat32Error::NotFound),
        None => Ok(None),
    }
}

/// Rejects traversal through an unknown namespace entry.
fn validate_components(path: &str) -> Result<(), Fat32Error> {
    let mut components: Vec<&str> = Vec::new();
    for component in path.split('/').filter(|component| !component.is_empty()) {
        if components.len() >= 2 && components[0] == DIRECTORY_NAME {
            return match components[1] {
                NULL_NAME => Err(Fat32Error::NotADirectory),
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
        path if path.starts_with(DIRECTORY_PREFIX) => Some(DevicePath::Missing),
        _ => None,
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_namespace_paths() {
        assert_eq!(resolve("/", "/dev"), Ok(Some(DevicePath::Directory)));
        assert_eq!(resolve("/", "/dev/null"), Ok(Some(DevicePath::Null)));
        assert_eq!(resolve("/", "/dev/unknown"), Ok(Some(DevicePath::Missing)));
        assert_eq!(resolve("/", "/dev/unknown/child"), Err(Fat32Error::NotFound));
        assert_eq!(resolve("/", "/dev/null/child"), Err(Fat32Error::NotADirectory));
    }

    #[test]
    fn synthesizes_stat() {
        let stat: Stat = stat("/", "/dev").expect("valid path").expect("existing path");
        assert_eq!(
            stat,
            Stat::new(
                0,
                true,
                STAT_TIMESTAMP_SECS,
                STAT_TIMESTAMP_SECS,
                STAT_TIMESTAMP_SECS,
            )
        );

        let stat: PosixStat =
            posix_stat("/", "/dev").expect("valid path").expect("existing path");
        assert_eq!(stat.st_dev, DEVICE_NAMESPACE_ID);
        assert_eq!(stat.st_ino, DIRECTORY_INODE);
        assert_eq!(stat.st_blksize, ::arch::mem::PAGE_SIZE as i64);
    }

    #[test]
    fn routing_ownership_is_distinct_from_validity() {
        assert!(owns("/", "/dev/unknown/child").expect("valid path"));
        assert!(!owns("/", "/dev/../tmp").expect("valid path"));
    }

    #[test]
    fn normalizes_paths_before_resolving() {
        assert_eq!(resolve("/", "/dev/"), Ok(Some(DevicePath::Directory)));
        assert_eq!(resolve("/", "dev"), Ok(Some(DevicePath::Directory)));
        assert_eq!(resolve("/", "/dev/../tmp"), Ok(None));
    }

    #[test]
    fn ignores_similar_prefixes() {
        assert_eq!(resolve("/", "/"), Ok(None));
        assert_eq!(resolve("/", "/device"), Ok(None));
        assert_eq!(resolve("/", "/devil/null"), Ok(None));
    }
}
