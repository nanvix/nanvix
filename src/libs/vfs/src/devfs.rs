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
    mount::{
        anchor_path,
        normalize_anchored,
    },
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
///
/// TODO (#3101): Remove this allowance when typed callers replace the legacy adapters.
#[allow(dead_code)]
pub(crate) fn resolve_anchored(path: AnchoredPath) -> Result<Option<DevicePath>, Fat32Error> {
    let absolute: String = path.into_absolute()?;
    resolve_absolute(&absolute)
}

/// Resolves a path in the synthetic device namespace.
///
/// TODO (#3101): Remove this working-directory adapter after callers pass [`AnchoredPath`].
pub(crate) fn resolve(cwd: &str, path: &str) -> Result<Option<DevicePath>, Fat32Error> {
    let absolute: String = anchor_path(path, cwd)?;
    resolve_absolute(&absolute)
}

/// Returns whether routing for a path belongs to the synthetic device namespace.
///
/// TODO (#3101): Remove this working-directory adapter after callers pass [`AnchoredPath`].
pub(crate) fn owns(cwd: &str, path: &str) -> Result<bool, Fat32Error> {
    let absolute: String = anchor_path(path, cwd)?;
    match resolve_absolute(&absolute) {
        Ok(path) => Ok(path.is_some()),
        Err(Fat32Error::NotFound | Fat32Error::NotADirectory) => Ok(true),
        Err(error) => Err(error),
    }
}

/// Resolves metadata for an existing device-namespace path.
///
/// TODO (#3101): Remove this working-directory adapter after callers pass [`AnchoredPath`].
fn metadata(cwd: &str, path: &str) -> Result<Option<DeviceMetadata>, Fat32Error> {
    let Some(device_path) = resolve(cwd, path)? else {
        return Ok(None);
    };
    let metadata: DeviceMetadata = device_path.metadata().ok_or(Fat32Error::NotFound)?;
    if path.ends_with('/') && !metadata.is_directory {
        return Err(Fat32Error::NotADirectory);
    }
    Ok(Some(metadata))
}

/// Synthesizes backend-neutral metadata for an existing devfs path.
///
/// TODO (#3101): Remove this working-directory adapter after callers pass [`AnchoredPath`].
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
///
/// TODO (#3101): Remove this working-directory adapter after callers pass [`AnchoredPath`].
pub(crate) fn posix_stat(cwd: &str, path: &str) -> Result<Option<PosixStat>, Fat32Error> {
    Ok(metadata(cwd, path)?.map(build_posix_stat))
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
///
/// TODO (#3101): Remove this working-directory adapter after callers pass [`AnchoredPath`].
pub(crate) fn read_dir(cwd: &str, path: &str) -> Result<Option<Vec<DirEntry>>, Fat32Error> {
    match resolve(cwd, path)? {
        Some(DevicePath::Directory) => Ok(Some(alloc::vec![
            DirEntry::new_character_device(String::from(NULL_NAME), DevFsInodeId::Null.into()),
            DirEntry::new_character_device(String::from(TTY_NAME), DevFsInodeId::Tty.into()),
            DirEntry::new_character_device(
                String::from(CONSOLE_NAME),
                DevFsInodeId::Console.into(),
            ),
        ])),
        Some(DevicePath::Null | DevicePath::Tty | DevicePath::Console) => {
            Err(Fat32Error::NotADirectory)
        },
        Some(DevicePath::Missing) => Err(Fat32Error::NotFound),
        None => Ok(None),
    }
}

/// Validates and classifies an absolute namespace path.
fn resolve_absolute(path: &str) -> Result<Option<DevicePath>, Fat32Error> {
    validate_components(path)?;
    let normalized: String = normalize_anchored(path);
    Ok(resolve_normalized(&normalized))
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
mod tests {
    use super::*;
    use crate::mount::{
        Mount,
        Vfs,
    };
    use ::fat32::{
        Fat,
        FatResolvedPath,
        RawMemoryStorage,
    };

    /// Creates a root FAT mount and keeps its backing allocation alive beside it.
    fn make_root_vfs() -> (Vfs, Vec<u8>) {
        let size: usize = 64 * 1024;
        let mut buffer: Vec<u8> = alloc::vec![0u8; size];
        let pointer: *mut u8 = buffer.as_mut_ptr();
        let mut storage: RawMemoryStorage =
            unsafe { RawMemoryStorage::new(pointer, size).expect("valid storage") };
        ::fatfs::format_volume(&mut storage, ::fatfs::FormatVolumeOptions::new())
            .expect("format should succeed");
        let fat: Fat = unsafe { Fat::from_memory(pointer, size).expect("valid FAT") };
        let mount: Mount = Mount::new(String::from("/"), fat, false).expect("valid mount");
        let mut vfs: Vfs = Vfs::new();
        vfs.add_mount(mount).expect("add root mount");
        (vfs, buffer)
    }

    #[test]
    fn anchored_resolution_uses_checked_paths() {
        let path: AnchoredPath = AnchoredPath::new(-99, String::from("/tmp/../dev//./null"))
            .expect("valid raw path should be accepted");
        assert_eq!(resolve_anchored(path), Ok(Some(DevicePath::Null)));

        assert_eq!(AnchoredPath::new(-99, String::new()), Err(Fat32Error::NotFound));

        let invalid: &str = "/tmp/invalid\0/../dev/null";
        assert_eq!(AnchoredPath::new(-99, String::from(invalid)), Err(Fat32Error::InvalidPath),);
        assert_eq!(resolve("/", invalid), Err(Fat32Error::InvalidPath));
    }

    #[test]
    fn legacy_resolution_anchors_relative_paths() {
        assert_eq!(resolve("/", "dev"), Ok(Some(DevicePath::Directory)));
        assert_eq!(resolve("/tmp/work", "../../dev//./console"), Ok(Some(DevicePath::Console)));
        assert_eq!(resolve("/", "/dev/../tmp"), Ok(None));
    }

    #[test]
    fn validates_traversal_through_device_nodes() {
        assert_eq!(resolve("/", "/dev/unknown"), Ok(Some(DevicePath::Missing)));
        assert_eq!(resolve("/", "/dev/unknown/child"), Err(Fat32Error::NotFound));
        assert_eq!(resolve("/", "/dev/null/../console"), Err(Fat32Error::NotADirectory));
        assert_eq!(resolve("/", "/dev/console/child"), Err(Fat32Error::NotADirectory));
    }

    #[test]
    fn synthesizes_stat_and_preserves_trailing_slashes() {
        let vfs_stat: Stat = stat("/", "/dev/")
            .expect("valid path")
            .expect("existing path");
        assert_eq!(
            vfs_stat,
            Stat::new(0, true, STAT_TIMESTAMP_SECS, STAT_TIMESTAMP_SECS, STAT_TIMESTAMP_SECS,)
        );

        let posix_stat: PosixStat = posix_stat("/", "/dev")
            .expect("valid path")
            .expect("existing path");
        assert_eq!(posix_stat.st_dev, FilesystemDeviceId::DevFs.into());
        assert_eq!(posix_stat.st_ino, DevFsInodeId::Directory.into());
        assert_eq!(posix_stat.st_blksize, ::arch::mem::PAGE_SIZE as i64);
        assert_eq!(stat("/", "/dev/null/"), Err(Fat32Error::NotADirectory));
    }

    #[test]
    fn reads_only_the_device_directory() {
        let entries: Vec<DirEntry> = read_dir("/", "/dev")
            .expect("valid path")
            .expect("devfs directory");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name(), NULL_NAME);
        assert_eq!(entries[1].name(), TTY_NAME);
        assert_eq!(entries[2].name(), CONSOLE_NAME);
        assert_eq!(read_dir("/", "/dev/null"), Err(Fat32Error::NotADirectory));
        assert_eq!(read_dir("/", "/dev/missing"), Err(Fat32Error::NotFound));
    }

    #[test]
    fn routing_ownership_is_distinct_from_validity() {
        assert!(owns("/", "/dev/unknown/child").expect("owned invalid path"));
        assert!(owns("/", "/dev/null/child").expect("owned invalid traversal"));
        assert!(!owns("/", "/dev/../tmp").expect("path outside devfs"));
    }

    #[test]
    fn ignores_similar_prefixes() {
        assert_eq!(resolve("/", "/"), Ok(None));
        assert_eq!(resolve("/", "/device"), Ok(None));
        assert_eq!(resolve("/", "/devil/null"), Ok(None));
    }

    #[test]
    fn routes_devfs_before_fat_and_preserves_fat_cache_parity() {
        let (mut vfs, _buffer) = make_root_vfs();
        let device_path: AnchoredPath = AnchoredPath::new(-99, String::from("/dev/null"))
            .expect("valid raw path should be accepted");

        assert_eq!(resolve_anchored(device_path), Ok(Some(DevicePath::Null)));
        assert_eq!(vfs.resolve_cache_len(), 0, "devfs routing must not consult FAT");

        let fat_path: AnchoredPath = AnchoredPath::new(-99, String::from("/data//./file"))
            .expect("valid raw path should be accepted");
        assert_eq!(resolve_anchored(fat_path.clone()), Ok(None));
        let first: FatResolvedPath = vfs.resolve(fat_path.clone()).expect("FAT fallthrough");
        let second: FatResolvedPath = vfs.resolve(fat_path).expect("cached FAT fallthrough");
        assert_eq!(first.as_str(), "data/file");
        assert_eq!(first, second, "cache hit must preserve the checked FAT path");
        assert_eq!(vfs.resolve_cache_len(), 1);
    }
}
