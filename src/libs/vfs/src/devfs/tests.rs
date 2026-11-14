// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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

fn resolve(path: &str) -> Result<Option<DevicePath>, Fat32Error> {
    resolve_anchored(AnchoredPath::new(-99, String::from(path))?)
}

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
    assert_eq!(resolve(invalid), Err(Fat32Error::InvalidPath));
}

#[test]
fn checked_resolution_normalizes_paths() {
    assert_eq!(resolve("/dev"), Ok(Some(DevicePath::Directory)));
    assert_eq!(resolve("/tmp/work/../../dev//./console"), Ok(Some(DevicePath::Console)));
    assert_eq!(resolve("/dev/../tmp"), Ok(None));
    assert_eq!(resolve("/dev//./stdin"), Ok(Some(DevicePath::Alias(DescriptorAlias::Stdin))));
    assert_eq!(resolve("/dev/stdout"), Ok(Some(DevicePath::Alias(DescriptorAlias::Stdout))));
    assert_eq!(resolve("/dev/stderr"), Ok(Some(DevicePath::Alias(DescriptorAlias::Stderr))));
}

#[test]
fn validates_traversal_through_device_nodes() {
    assert_eq!(resolve("/dev/unknown"), Ok(Some(DevicePath::Missing)));
    assert_eq!(resolve("/dev/unknown/child"), Err(Fat32Error::NotFound));
    assert_eq!(resolve("/dev/null/../console"), Err(Fat32Error::NotADirectory));
    assert_eq!(resolve("/dev/console/child"), Err(Fat32Error::NotADirectory));
    assert_eq!(resolve("/dev/stdin/child"), Err(Fat32Error::NotADirectory));
    assert_eq!(resolve("/dev/stderr/../null"), Err(Fat32Error::NotADirectory));
}

#[test]
fn synthesizes_stat_for_classified_devices() {
    let vfs_stat: Stat = stat(DevicePath::Directory).expect("existing path");
    assert_eq!(
        vfs_stat,
        Stat::new(0, true, STAT_TIMESTAMP_SECS, STAT_TIMESTAMP_SECS, STAT_TIMESTAMP_SECS,)
    );

    let posix_stat: PosixStat = posix_stat(DevicePath::Directory).expect("existing path");
    assert_eq!(posix_stat.st_dev, FilesystemDeviceId::DevFs.into());
    assert_eq!(posix_stat.st_ino, DevFsInodeId::Directory.into());
    assert_eq!(posix_stat.st_blksize, ::arch::mem::PAGE_SIZE as i64);
    assert_eq!(stat(DevicePath::Missing), Err(Fat32Error::NotFound));
}

#[test]
fn reads_only_the_device_directory() {
    let entries: Vec<DirEntry> = read_dir(DevicePath::Directory).expect("devfs directory");
    let names: Vec<&str> = entries.iter().map(DirEntry::name).collect();
    assert_eq!(names, ["null", "tty", "console", "stdin", "stdout", "stderr"]);
    assert!(entries[..3].iter().all(DirEntry::is_character_device));
    assert!(entries[3..]
        .iter()
        .all(|entry| entry.is_descriptor_alias() && !entry.is_character_device()));
    assert_eq!(read_dir(DevicePath::Null), Err(Fat32Error::NotADirectory));
    assert_eq!(read_dir(DevicePath::Alias(DescriptorAlias::Stdin)), Err(Fat32Error::NotADirectory));
    assert_eq!(read_dir(DevicePath::Missing), Err(Fat32Error::NotFound));
}

#[test]
fn ignores_similar_prefixes() {
    assert_eq!(resolve("/"), Ok(None));
    assert_eq!(resolve("/device"), Ok(None));
    assert_eq!(resolve("/devil/null"), Ok(None));
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
